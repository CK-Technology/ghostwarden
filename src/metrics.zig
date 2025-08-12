const std = @import("std");
const Allocator = std.mem.Allocator;
const Config = @import("config.zig");

pub const MetricsCollector = struct {
    allocator: Allocator,
    config: Config.Config,
    server: ?std.http.Server = null,
    
    // Counter metrics
    bans_total: u64 = 0,
    unbans_total: u64 = 0,
    crowdsec_decisions_total: u64 = 0,
    wazuh_alerts_total: u64 = 0,
    proxmox_api_calls_total: u64 = 0,
    nftables_operations_total: u64 = 0,
    
    // Error counters
    crowdsec_errors_total: u64 = 0,
    wazuh_errors_total: u64 = 0,
    proxmox_errors_total: u64 = 0,
    nftables_errors_total: u64 = 0,
    
    // Gauge metrics
    currently_banned_ips: u64 = 0,
    sync_duration_seconds: f64 = 0.0,
    last_sync_timestamp: i64 = 0,
    
    // Histogram-like metrics (simple buckets for now)
    sync_duration_bucket_1s: u64 = 0,    // < 1s
    sync_duration_bucket_5s: u64 = 0,    // 1-5s
    sync_duration_bucket_10s: u64 = 0,   // 5-10s
    sync_duration_bucket_inf: u64 = 0,   // > 10s
    
    const Self = @This();

    pub fn init(allocator: Allocator, config: Config.Config) Self {
        return Self{
            .allocator = allocator,
            .config = config,
        };
    }

    pub fn deinit(self: *Self) void {
        _ = self; // Unused for now
        // Simplified metrics collection - no HTTP server currently
        std.debug.print("Metrics collection stopped\n", .{});
    }

    pub fn startMetricsServer(self: *Self) !void {
        if (!self.config.daemon.metrics_enabled) {
            return;
        }

        std.log.info("Prometheus metrics collection enabled", .{});
        std.log.info("Metrics will be logged periodically to stdout", .{});
        std.log.info("Future versions will include HTTP metrics endpoint on {s}:{d}", .{ 
            self.config.daemon.bind_address, self.config.daemon.bind_port 
        });
    }

    // TODO: Implement full HTTP server for metrics endpoint in future version
    // For now, metrics are collected but not exposed via HTTP
    //
    // fn handleMetricsRequests(self: *Self) void {
    //     // Implementation would go here
    // }
    //
    // fn handleMetricsRequest(self: *Self, response: *std.http.Server.Response) !void {
    //     // Implementation would go here
    // }

    fn generatePrometheusMetrics(self: *Self) ![]u8 {
        var buffer = std.ArrayList(u8).init(self.allocator);
        const writer = buffer.writer();

        // Write help and type information
        try writer.print(
            \\# HELP ghostwarden_bans_total Total number of IP bans applied
            \\# TYPE ghostwarden_bans_total counter
            \\ghostwarden_bans_total {}
            \\
            \\# HELP ghostwarden_unbans_total Total number of IP unbans applied
            \\# TYPE ghostwarden_unbans_total counter
            \\ghostwarden_unbans_total {}
            \\
            \\# HELP ghostwarden_crowdsec_decisions_total Total decisions received from CrowdSec
            \\# TYPE ghostwarden_crowdsec_decisions_total counter
            \\ghostwarden_crowdsec_decisions_total {}
            \\
            \\# HELP ghostwarden_wazuh_alerts_total Total alerts received from Wazuh
            \\# TYPE ghostwarden_wazuh_alerts_total counter
            \\ghostwarden_wazuh_alerts_total {}
            \\
            \\# HELP ghostwarden_proxmox_api_calls_total Total API calls made to Proxmox
            \\# TYPE ghostwarden_proxmox_api_calls_total counter
            \\ghostwarden_proxmox_api_calls_total {}
            \\
            \\# HELP ghostwarden_nftables_operations_total Total nftables operations performed
            \\# TYPE ghostwarden_nftables_operations_total counter
            \\ghostwarden_nftables_operations_total {}
            \\
            , .{ 
                self.bans_total,
                self.unbans_total,
                self.crowdsec_decisions_total,
                self.wazuh_alerts_total,
                self.proxmox_api_calls_total,
                self.nftables_operations_total
            });

        // Error counters
        try writer.print(
            \\# HELP ghostwarden_errors_total Total errors by component
            \\# TYPE ghostwarden_errors_total counter
            \\ghostwarden_errors_total{{component="crowdsec"}} {}
            \\ghostwarden_errors_total{{component="wazuh"}} {}
            \\ghostwarden_errors_total{{component="proxmox"}} {}
            \\ghostwarden_errors_total{{component="nftables"}} {}
            \\
            , .{
                self.crowdsec_errors_total,
                self.wazuh_errors_total,
                self.proxmox_errors_total,
                self.nftables_errors_total
            });

        // Gauge metrics
        try writer.print(
            \\# HELP ghostwarden_banned_ips_current Currently banned IP addresses
            \\# TYPE ghostwarden_banned_ips_current gauge
            \\ghostwarden_banned_ips_current {}
            \\
            \\# HELP ghostwarden_sync_duration_seconds Duration of last sync operation
            \\# TYPE ghostwarden_sync_duration_seconds gauge
            \\ghostwarden_sync_duration_seconds {d:.3}
            \\
            \\# HELP ghostwarden_last_sync_timestamp Unix timestamp of last successful sync
            \\# TYPE ghostwarden_last_sync_timestamp gauge
            \\ghostwarden_last_sync_timestamp {}
            \\
            , .{
                self.currently_banned_ips,
                self.sync_duration_seconds,
                self.last_sync_timestamp
            });

        // Histogram buckets for sync duration
        try writer.print(
            \\# HELP ghostwarden_sync_duration_bucket Sync duration histogram buckets
            \\# TYPE ghostwarden_sync_duration_bucket histogram
            \\ghostwarden_sync_duration_bucket{{le="1.0"}} {}
            \\ghostwarden_sync_duration_bucket{{le="5.0"}} {}
            \\ghostwarden_sync_duration_bucket{{le="10.0"}} {}
            \\ghostwarden_sync_duration_bucket{{le="+Inf"}} {}
            \\
            , .{
                self.sync_duration_bucket_1s,
                self.sync_duration_bucket_5s,
                self.sync_duration_bucket_10s,
                self.sync_duration_bucket_inf
            });

        // Build information
        try writer.print(
            \\# HELP ghostwarden_build_info Build information
            \\# TYPE ghostwarden_build_info gauge
            \\ghostwarden_build_info{{version="0.1.0",zig_version="0.15.0"}} 1
            \\
            \\# HELP ghostwarden_up Whether Ghostwarden is up and running
            \\# TYPE ghostwarden_up gauge
            \\ghostwarden_up 1
            \\
            );

        return buffer.toOwnedSlice();
    }

    // Metric recording methods
    pub fn recordBan(self: *Self) void {
        self.bans_total += 1;
    }

    pub fn recordUnban(self: *Self) void {
        self.unbans_total += 1;
    }

    pub fn recordCrowdSecDecision(self: *Self) void {
        self.crowdsec_decisions_total += 1;
    }

    pub fn recordWazuhAlert(self: *Self) void {
        self.wazuh_alerts_total += 1;
    }

    pub fn recordProxmoxApiCall(self: *Self) void {
        self.proxmox_api_calls_total += 1;
    }

    pub fn recordNftablesOperation(self: *Self) void {
        self.nftables_operations_total += 1;
    }

    pub fn recordCrowdSecError(self: *Self) void {
        self.crowdsec_errors_total += 1;
    }

    pub fn recordWazuhError(self: *Self) void {
        self.wazuh_errors_total += 1;
    }

    pub fn recordProxmoxError(self: *Self) void {
        self.proxmox_errors_total += 1;
    }

    pub fn recordNftablesError(self: *Self) void {
        self.nftables_errors_total += 1;
    }

    pub fn updateCurrentlyBannedIps(self: *Self, count: u64) void {
        self.currently_banned_ips = count;
    }

    pub fn recordSyncDuration(self: *Self, duration_seconds: f64) void {
        self.sync_duration_seconds = duration_seconds;
        self.last_sync_timestamp = std.time.timestamp();

        // Update histogram buckets
        if (duration_seconds <= 1.0) {
            self.sync_duration_bucket_1s += 1;
        } else if (duration_seconds <= 5.0) {
            self.sync_duration_bucket_5s += 1;
        } else if (duration_seconds <= 10.0) {
            self.sync_duration_bucket_10s += 1;
        } else {
            self.sync_duration_bucket_inf += 1;
        }
    }

    pub fn logMetricsSummary(self: *Self) void {
        std.log.info("=== Ghostwarden Metrics Summary ===", .{});
        std.log.info("Bans: {} | Unbans: {} | Currently banned: {}", .{ self.bans_total, self.unbans_total, self.currently_banned_ips });
        std.log.info("CrowdSec decisions: {} | Wazuh alerts: {}", .{ self.crowdsec_decisions_total, self.wazuh_alerts_total });
        std.log.info("Proxmox calls: {} | NFTables ops: {}", .{ self.proxmox_api_calls_total, self.nftables_operations_total });
        std.log.info("Errors - CrowdSec: {} | Wazuh: {} | Proxmox: {} | NFTables: {}", .{ 
            self.crowdsec_errors_total, self.wazuh_errors_total, self.proxmox_errors_total, self.nftables_errors_total 
        });
        std.log.info("Last sync duration: {d:.2}s | Last sync: {d}", .{ self.sync_duration_seconds, self.last_sync_timestamp });
    }
};

test "metrics collector init" {
    const config = Config.Config{
        .pve = .{
            .api_url = "https://test.com:8006/api2/json",
            .token_id = "test",
            .token_secret = "secret",
        },
        .daemon = .{
            .metrics_enabled = true,
        },
    };

    var collector = MetricsCollector.init(std.testing.allocator, config);
    defer collector.deinit();

    collector.recordBan();
    collector.recordCrowdSecDecision();
    collector.updateCurrentlyBannedIps(42);

    try std.testing.expect(collector.bans_total == 1);
    try std.testing.expect(collector.crowdsec_decisions_total == 1);
    try std.testing.expect(collector.currently_banned_ips == 42);
}
