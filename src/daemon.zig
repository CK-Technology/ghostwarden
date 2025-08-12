const std = @import("std");
const Allocator = std.mem.Allocator;
const Config = @import("config.zig");
const CrowdSec = @import("crowdsec.zig");
const Wazuh = @import("wazuh.zig");
const Proxmox = @import("proxmox.zig");
const Nftables = @import("nftables.zig");
const Metrics = @import("metrics.zig");

pub const GhostWardenDaemon = struct {
    allocator: Allocator,
    config: Config.Config,
    crowdsec_client: ?CrowdSec.CrowdSecClient = null,
    wazuh_client: ?Wazuh.WazuhClient = null,
    proxmox_client: Proxmox.ProxmoxClient,
    nftables_client: ?Nftables.NftablesClient = null,
    metrics_collector: ?Metrics.MetricsCollector = null,
    running: bool = false,
    last_sync: i64 = 0,

    const Self = @This();

    pub fn init(allocator: Allocator, config: Config.Config) !Self {
        var daemon = Self{
            .allocator = allocator,
            .config = config,
            .proxmox_client = Proxmox.ProxmoxClient.init(allocator, config.pve),
        };

        // Initialize CrowdSec client if configured
        if (config.crowdsec) |crowdsec_config| {
            daemon.crowdsec_client = try CrowdSec.CrowdSecClient.init(allocator, crowdsec_config);
        }

        // Initialize Wazuh client if configured
        if (config.wazuh) |wazuh_config| {
            daemon.wazuh_client = Wazuh.WazuhClient.init(allocator, wazuh_config);
        }

        // Initialize NFTables client if enabled
        if (config.nftables.enabled) {
            daemon.nftables_client = try Nftables.NftablesClient.init(allocator, config.nftables);
        }

        // Initialize metrics collector if enabled
        if (config.daemon.metrics_enabled) {
            daemon.metrics_collector = Metrics.MetricsCollector.init(allocator, config);
        }

        return daemon;
    }

    pub fn deinit(self: *Self) void {
        if (self.crowdsec_client) |*client| {
            client.deinit();
        }
        if (self.wazuh_client) |*client| {
            client.deinit();
        }
        self.proxmox_client.deinit();
        if (self.nftables_client) |*client| {
            client.deinit();
        }
        if (self.metrics_collector) |*collector| {
            collector.deinit();
        }
    }

    pub fn start(self: *Self) !void {
        std.log.info("Starting Ghostwarden daemon...", .{});
        
        // Test connections
        try self.testConnections();
        
        // Start metrics server if enabled
        if (self.metrics_collector) |*collector| {
            try collector.startMetricsServer();
            std.log.info("Prometheus metrics available at http://{s}:{d}/metrics", .{ self.config.daemon.bind_address, self.config.daemon.bind_port });
        }
        
        self.running = true;
        self.last_sync = std.time.timestamp();

        // Start main event loop
        var sync_counter: u32 = 0;
        while (self.running) {
            const sync_start = std.time.timestamp();
            const now = sync_start;
            const time_since_sync = now - self.last_sync;

            if (time_since_sync >= self.config.daemon.sync_interval_seconds) {
                try self.syncBans();
                self.last_sync = now;
                sync_counter += 1;
                
                // Record sync duration metrics
                if (self.metrics_collector) |*collector| {
                    const sync_duration = @as(f64, @floatFromInt(std.time.timestamp() - sync_start));
                    collector.recordSyncDuration(sync_duration);
                    
                    // Log metrics summary every 10 syncs (roughly every 10 minutes with default config)
                    if (sync_counter % 10 == 0) {
                        collector.logMetricsSummary();
                    }
                }
            }

            // Sleep for 1 second before next iteration
            std.time.sleep(1 * std.time.ns_per_s);
        }
    }

    pub fn stop(self: *Self) void {
        std.log.info("Stopping Ghostwarden daemon...", .{});
        self.running = false;
    }

    fn testConnections(self: *Self) !void {
        std.log.info("Testing connections...", .{});

        // Test Proxmox connection
        self.proxmox_client.testConnection() catch |err| {
            std.log.err("Failed to connect to Proxmox VE: {}", .{err});
            return err;
        };
        std.log.info("✓ Proxmox VE connection successful", .{});

        // Test CrowdSec connection if configured
        if (self.crowdsec_client) |*client| {
            client.heartbeat() catch |err| {
                std.log.warn("CrowdSec connection test failed: {}", .{err});
            };
        }

        // Test Wazuh connection if configured
        if (self.wazuh_client) |*client| {
            client.authenticate() catch |err| {
                std.log.warn("Wazuh connection test failed: {}", .{err});
            };
        }
    }

    fn syncBans(self: *Self) !void {
        std.log.debug("Starting ban synchronization...", .{});

        var ips_to_ban = std.ArrayList([]const u8).init(self.allocator);
        defer ips_to_ban.deinit();
        var ips_to_unban = std.ArrayList([]const u8).init(self.allocator);
        defer ips_to_unban.deinit();

        // Collect decisions from CrowdSec
        if (self.crowdsec_client) |*client| {
            const decisions = client.getDecisions(false) catch |err| {
                std.log.err("Failed to get CrowdSec decisions: {}", .{err});
                if (self.metrics_collector) |*collector| {
                    collector.recordCrowdSecError();
                }
                return;
            };

            if (decisions.new) |new_decisions| {
                for (new_decisions) |decision| {
                    if (self.metrics_collector) |*collector| {
                        collector.recordCrowdSecDecision();
                    }
                    
                    if (std.mem.eql(u8, decision.type, "ban") and std.mem.eql(u8, decision.scope, "Ip")) {
                        if (!self.isWhitelisted(decision.value)) {
                            try ips_to_ban.append(try self.allocator.dupe(u8, decision.value));
                            std.log.info("CrowdSec ban: {s} (scenario: {s})", .{ decision.value, decision.scenario });
                            if (self.metrics_collector) |*collector| {
                                collector.recordBan();
                            }
                        }
                    }
                }
            }

            if (decisions.deleted) |deleted_decisions| {
                for (deleted_decisions) |decision| {
                    if (std.mem.eql(u8, decision.type, "ban") and std.mem.eql(u8, decision.scope, "Ip")) {
                        try ips_to_unban.append(try self.allocator.dupe(u8, decision.value));
                        std.log.info("CrowdSec unban: {s}", .{decision.value});
                        if (self.metrics_collector) |*collector| {
                            collector.recordUnban();
                        }
                    }
                }
            }
        }

        // Collect alerts from Wazuh
        if (self.wazuh_client) |*client| {
            const alerts = client.getAlerts(null, 100) catch |err| {
                std.log.err("Failed to get Wazuh alerts: {}", .{err});
                if (self.metrics_collector) |*collector| {
                    collector.recordWazuhError();
                }
                return;
            };

            const actions = client.convertToActions(alerts) catch |err| {
                std.log.err("Failed to convert Wazuh alerts: {}", .{err});
                if (self.metrics_collector) |*collector| {
                    collector.recordWazuhError();
                }
                return;
            };

            for (actions) |action| {
                if (self.metrics_collector) |*collector| {
                    collector.recordWazuhAlert();
                }
                
                switch (action.action) {
                    .ban => {
                        if (!self.isWhitelisted(action.source_ip)) {
                            try ips_to_ban.append(try self.allocator.dupe(u8, action.source_ip));
                            std.log.info("Wazuh ban: {s} (rule: {})", .{ action.source_ip, action.rule_id });
                            if (self.metrics_collector) |*collector| {
                                collector.recordBan();
                            }
                        }
                    },
                    .allow => {
                        try ips_to_unban.append(try self.allocator.dupe(u8, action.source_ip));
                        std.log.info("Wazuh unban: {s}", .{action.source_ip});
                        if (self.metrics_collector) |*collector| {
                            collector.recordUnban();
                        }
                    },
                    .monitor => {
                        // Just log, no action
                        std.log.debug("Wazuh monitor: {s}", .{action.source_ip});
                    },
                }
            }
        }

        // Apply bans to Proxmox
        if (ips_to_ban.items.len > 0 or ips_to_unban.items.len > 0) {
            if (self.metrics_collector) |*collector| {
                collector.recordProxmoxApiCall();
            }
            
            self.proxmox_client.bulkUpdateIpSet(
                self.config.pve.ipset_name,
                ips_to_ban.items,
                ips_to_unban.items,
            ) catch |err| {
                std.log.err("Failed to update Proxmox IPSet: {}", .{err});
                if (self.metrics_collector) |*collector| {
                    collector.recordProxmoxError();
                }
            };
        }

        // Apply bans to local NFTables if enabled
        if (self.nftables_client) |*client| {
            for (ips_to_ban.items) |ip| {
                if (self.metrics_collector) |*collector| {
                    collector.recordNftablesOperation();
                }
                client.addIp(ip) catch |err| {
                    std.log.warn("Failed to add IP {s} to nftables: {}", .{ ip, err });
                    if (self.metrics_collector) |*collector| {
                        collector.recordNftablesError();
                    }
                };
            }

            for (ips_to_unban.items) |ip| {
                if (self.metrics_collector) |*collector| {
                    collector.recordNftablesOperation();
                }
                client.removeIp(ip) catch |err| {
                    std.log.warn("Failed to remove IP {s} from nftables: {}", .{ ip, err });
                    if (self.metrics_collector) |*collector| {
                        collector.recordNftablesError();
                    }
                };
            }
        }

        // Update currently banned IPs metric
        if (self.metrics_collector) |*collector| {
            // Try to get current count from nftables or approximation
            const current_count = self.getCurrentBannedCount();
            collector.updateCurrentlyBannedIps(current_count);
        }

        std.log.debug("Ban synchronization completed: +{d} -{d}", .{ ips_to_ban.items.len, ips_to_unban.items.len });

        // Cleanup allocated strings
        for (ips_to_ban.items) |ip| {
            self.allocator.free(ip);
        }
        for (ips_to_unban.items) |ip| {
            self.allocator.free(ip);
        }
    }

    fn isWhitelisted(self: *Self, ip: []const u8) bool {
        for (self.config.whitelist) |whitelist_entry| {
            if (std.mem.eql(u8, ip, whitelist_entry)) {
                std.log.debug("IP {s} is whitelisted", .{ip});
                return true;
            }
        }
        return false;
    }

    fn getCurrentBannedCount(self: *Self) u64 {
        if (self.nftables_client) |*client| {
            const ips = client.listIps() catch |err| {
                std.log.debug("Failed to get nftables IP count: {}", .{err});
                return 0;
            };
            defer {
                for (ips) |ip| {
                    self.allocator.free(ip);
                }
                self.allocator.free(ips);
            }
            return @intCast(ips.len);
        }
        return 0;
    }
};

test "daemon init" {
    const config = Config.Config{
        .pve = .{
            .api_url = "https://test.com:8006/api2/json",
            .token_id = "test",
            .token_secret = "secret",
        },
    };

    var daemon = try GhostWardenDaemon.init(std.testing.allocator, config);
    defer daemon.deinit();
}
