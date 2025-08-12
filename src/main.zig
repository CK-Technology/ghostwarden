const std = @import("std");
const ghostwarden = @import("ghostwarden");
const toml = @import("toml");

const Config = ghostwarden.Config;
const Daemon = ghostwarden.Daemon;

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    // Parse command line arguments
    const args = try std.process.argsAlloc(allocator);
    defer std.process.argsFree(allocator, args);

    var config_path: ?[]const u8 = null;
    var show_help = false;
    var show_version = false;

    var i: usize = 1;
    while (i < args.len) : (i += 1) {
        const arg = args[i];
        if (std.mem.eql(u8, arg, "--help") or std.mem.eql(u8, arg, "-h")) {
            show_help = true;
        } else if (std.mem.eql(u8, arg, "--version") or std.mem.eql(u8, arg, "-v")) {
            show_version = true;
        } else if (std.mem.eql(u8, arg, "--config") or std.mem.eql(u8, arg, "-c")) {
            if (i + 1 >= args.len) {
                std.log.err("--config requires a file path", .{});
                return;
            }
            i += 1;
            config_path = args[i];
        } else if (arg[0] == '-') {
            std.log.err("Unknown option: {s}", .{arg});
            show_help = true;
        }
    }

    if (show_version) {
        std.debug.print("Ghostwarden v0.1.0\n", .{});
        return;
    }

    if (show_help) {
        printHelp();
        return;
    }

    // Default config path
    const final_config_path = config_path orelse "/etc/ghostwarden/config.toml";

    // Load configuration
    const config = Config.Config.loadFromFile(allocator, final_config_path) catch |err| switch (err) {
        error.FileNotFound => {
            std.log.err("Configuration file not found: {s}", .{final_config_path});
            std.log.info("Create a configuration file or use --config to specify a different path", .{});
            return;
        },
        error.ParseError => {
            std.log.err("Failed to parse configuration file: {s}", .{final_config_path});
            return;
        },
        else => {
            std.log.err("Failed to load configuration: {}", .{err});
            return;
        },
    };

    // Validate configuration
    try validateConfig(config);

    std.log.info("Starting Ghostwarden with config: {s}", .{final_config_path});

    // Initialize and start daemon
    var daemon = Daemon.GhostWardenDaemon.init(allocator, config) catch |err| {
        std.log.err("Failed to initialize daemon: {}", .{err});
        return;
    };
    defer daemon.deinit();

    // Set up signal handling for graceful shutdown
    const signal_handler = struct {
        var daemon_ptr: ?*Daemon.GhostWardenDaemon = null;

        fn handleSignal(sig: i32) callconv(.C) void {
            _ = sig;
            if (daemon_ptr) |d| {
                d.stop();
            }
        }
    };

    // TODO: Add proper signal handling for graceful shutdown
    // For now, just start the daemon
    signal_handler.daemon_ptr = &daemon;

    // Start daemon (blocking)
    daemon.start() catch |err| {
        std.log.err("Daemon failed: {}", .{err});
        return;
    };

    std.log.info("Ghostwarden stopped", .{});
}

fn printHelp() void {
    std.debug.print(
        \\Ghostwarden - Proxmox SDN + Cluster Firewall Enforcement
        \\
        \\USAGE:
        \\    ghostwarden [OPTIONS]
        \\
        \\OPTIONS:
        \\    -c, --config <FILE>    Configuration file path [default: /etc/ghostwarden/config.toml]
        \\    -h, --help             Print help information
        \\    -v, --version          Print version information
        \\
        \\EXAMPLES:
        \\    ghostwarden --config /path/to/config.toml
        \\    ghostwarden -c ./ghostwarden.toml
        \\
        \\For more information, see: https://github.com/CK-Technology/ghostwarden
        \\
    , .{});
}

fn validateConfig(config: Config.Config) !void {
    // Validate Proxmox configuration
    if (config.pve.api_url.len == 0) {
        std.log.err("Proxmox API URL is required", .{});
        return error.ConfigurationError;
    }
    if (config.pve.token_id.len == 0) {
        std.log.err("Proxmox token ID is required", .{});
        return error.ConfigurationError;
    }
    if (config.pve.token_secret.len == 0) {
        std.log.err("Proxmox token secret is required", .{});
        return error.ConfigurationError;
    }

    // Validate at least one source is configured
    if (config.crowdsec == null and config.wazuh == null) {
        std.log.err("At least one of CrowdSec or Wazuh must be configured", .{});
        return error.ConfigurationError;
    }

    // Validate CrowdSec config if present
    if (config.crowdsec) |crowdsec_config| {
        try ghostwarden.CrowdSec.CrowdSecClient.validateConfig(crowdsec_config);
    }

    std.log.info("Configuration validated successfully", .{});
}

test "main" {
    // Basic test to ensure main compiles
}

test "config validation" {
    const config = Config.Config{
        .pve = .{
            .api_url = "https://test.com:8006/api2/json",
            .token_id = "test",
            .token_secret = "secret",
        },
        .crowdsec = .{
            .lapi_url = "http://localhost:8080",
            .api_key = "test-key",
        },
    };

    try validateConfig(config);
}
