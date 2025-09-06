const std = @import("std");
const Allocator = std.mem.Allocator;

pub const ProxmoxConfig = struct {
    api_url: []const u8,
    token_id: []const u8,
    token_secret: []const u8,
    ipset_name: []const u8 = "ghostwarden-banned",
    node_name: ?[]const u8 = null,
    verify_ssl: bool = true,
};

pub const CrowdSecConfig = struct {
    lapi_url: []const u8,
    api_key: []const u8,
    machine_id: ?[]const u8 = null,
    scenarios: []const []const u8 = &.{},
    poll_interval_seconds: u32 = 30,
};

pub const WazuhConfig = struct {
    api_url: []const u8,
    api_user: []const u8,
    api_pass: []const u8,
    rules: []const []const u8 = &.{},
    verify_ssl: bool = true,
};

pub const NftablesConfig = struct {
    set_name: []const u8 = "ghostwarden-banned",
    table_name: []const u8 = "inet",
    chain_name: []const u8 = "input",
    enabled: bool = true,
};

pub const Config = struct {
    pve: ProxmoxConfig,
    crowdsec: ?CrowdSecConfig = null,
    wazuh: ?WazuhConfig = null,
    nftables: NftablesConfig = .{},
    whitelist: []const []const u8 = &.{},
    default_ban_duration_seconds: u32 = 3600,
    log_level: LogLevel = .info,
    daemon: DaemonConfig = .{},

    const DaemonConfig = struct {
        sync_interval_seconds: u32 = 60,
        max_retries: u32 = 3,
        bind_address: []const u8 = "127.0.0.1",
        bind_port: u16 = 8080,
        metrics_enabled: bool = false,
    };

    const LogLevel = enum {
        debug,
        info,
        warn,
        err,
    };

    pub fn loadFromFile(allocator: Allocator, path: []const u8) !Config {
        const file = std.fs.cwd().openFile(path, .{}) catch |err| switch (err) {
            error.FileNotFound => {
                std.log.err("Configuration file not found: {s}", .{path});
                return error.FileNotFound;
            },
            else => return err,
        };
        defer file.close();

        const file_size = try file.getEndPos();
        const content = try allocator.alloc(u8, file_size);
        _ = try file.readAll(content);
        defer allocator.free(content);

        return parseToml(allocator, content);
    }

    fn parseToml(allocator: Allocator, content: []const u8) !Config {
        const toml = @import("toml");
        
        var parser = toml.Parser(Config).init(allocator);
        defer parser.deinit();
        
        const parsed = try parser.parseString(content);
        return parsed.value;
    }

    pub fn validate(self: *const Config) !void {
        if (self.pve.api_url.len == 0) return error.ConfigurationError;
        if (self.pve.token_id.len == 0) return error.ConfigurationError;
        if (self.pve.token_secret.len == 0) return error.ConfigurationError;
        
        if (self.crowdsec == null and self.wazuh == null) {
            std.log.err("At least one of CrowdSec or Wazuh must be configured", .{});
            return error.ConfigurationError;
        }

        if (self.crowdsec) |cs| {
            if (cs.lapi_url.len == 0 or cs.api_key.len == 0) {
                return error.ConfigurationError;
            }
        }

        if (self.wazuh) |w| {
            if (w.api_url.len == 0 or w.api_user.len == 0 or w.api_pass.len == 0) {
                return error.ConfigurationError;
            }
        }
    }
};

test "config validation" {
    const config = Config{
        .pve = .{
            .api_url = "https://pve.example.com:8006/api2/json",
            .token_id = "root@pam!ghostwarden",
            .token_secret = "test-secret",
        },
        .crowdsec = .{
            .lapi_url = "http://crowdsec:8080",
            .api_key = "test-key",
        },
    };

    try config.validate();
}