const std = @import("std");
const Allocator = std.mem.Allocator;
const Config = @import("config.zig");

pub const NftablesClient = struct {
    allocator: Allocator,
    config: Config.NftablesConfig,

    const Self = @This();

    pub fn init(allocator: Allocator, config: Config.NftablesConfig) !Self {
        const client = Self{
            .allocator = allocator,
            .config = config,
        };

        // Ensure the nftables set exists
        try client.ensureSetExists();

        return client;
    }

    pub fn deinit(self: *Self) void {
        _ = self;
    }

    fn ensureSetExists(self: Self) !void {
        // Check if set exists
        const check_cmd = try std.fmt.allocPrint(self.allocator, "nft list set {s} {s} {s} 2>/dev/null", .{ 
            self.config.table_name, 
            self.config.table_name, 
            self.config.set_name 
        });
        defer self.allocator.free(check_cmd);

        const result = std.process.Child.run(.{
            .allocator = self.allocator,
            .argv = &[_][]const u8{ "sh", "-c", check_cmd },
        }) catch |err| {
            std.log.err("Failed to check nftables set existence: {}", .{err});
            return;
        };

        if (result.stderr.len > 0) {
            self.allocator.free(result.stderr);
        }
        self.allocator.free(result.stdout);

        // If command failed, create the set
        if (result.term.Exited != 0) {
            try self.createSet();
        }
    }

    fn createSet(self: Self) !void {
        // Create table if it doesn't exist
        const table_cmd = try std.fmt.allocPrint(self.allocator, "nft add table {s} {s} || true", .{ 
            self.config.table_name, 
            self.config.table_name 
        });
        defer self.allocator.free(table_cmd);

        const table_result = std.process.Child.run(.{
            .allocator = self.allocator,
            .argv = &[_][]const u8{ "sh", "-c", table_cmd },
        }) catch |err| {
            std.log.err("Failed to create nftables table: {}", .{err});
            return err;
        };
        defer {
            self.allocator.free(table_result.stdout);
            self.allocator.free(table_result.stderr);
        }

        if (table_result.term.Exited != 0) {
            std.log.err("Failed to create table {s}: {s}", .{ self.config.table_name, table_result.stderr });
            return error.NftablesError;
        }

        // Create the IP set
        const set_cmd = try std.fmt.allocPrint(self.allocator, "nft add set {s} {s} {s} {{ type ipv4_addr\\; flags interval\\; }}", .{ 
            self.config.table_name,
            self.config.table_name, 
            self.config.set_name 
        });
        defer self.allocator.free(set_cmd);

        const set_result = std.process.Child.run(.{
            .allocator = self.allocator,
            .argv = &[_][]const u8{ "sh", "-c", set_cmd },
        }) catch |err| {
            std.log.err("Failed to create nftables set: {}", .{err});
            return err;
        };
        defer {
            self.allocator.free(set_result.stdout);
            self.allocator.free(set_result.stderr);
        }

        if (set_result.term.Exited != 0) {
            std.log.err("Failed to create set {s}: {s}", .{ self.config.set_name, set_result.stderr });
            return error.NftablesError;
        }

        // Create a basic drop rule that references our set
        const rule_cmd = try std.fmt.allocPrint(self.allocator, 
            "nft add rule {s} {s} {s} ip saddr @{s} drop || true", 
            .{ 
                self.config.table_name, 
                self.config.table_name, 
                self.config.chain_name, 
                self.config.set_name 
            });
        defer self.allocator.free(rule_cmd);

        const rule_result = std.process.Child.run(.{
            .allocator = self.allocator,
            .argv = &[_][]const u8{ "sh", "-c", rule_cmd },
        }) catch |err| {
            std.log.warn("Failed to create nftables drop rule: {}", .{err});
            // Don't return error, rule might already exist
            return;
        };
        defer {
            self.allocator.free(rule_result.stdout);
            self.allocator.free(rule_result.stderr);
        }

        std.log.info("Created nftables set {s}.{s}.{s}", .{ 
            self.config.table_name, 
            self.config.table_name, 
            self.config.set_name 
        });
    }

    pub fn addIp(self: Self, ip: []const u8) !void {
        if (!self.isValidIp(ip)) {
            std.log.warn("Invalid IP address format: {s}", .{ip});
            return error.InvalidIpAddress;
        }

        const cmd = try std.fmt.allocPrint(self.allocator, "nft add element {s} {s} {s} {{ {s} }}", .{ 
            self.config.table_name, 
            self.config.table_name, 
            self.config.set_name, 
            ip 
        });
        defer self.allocator.free(cmd);

        const result = std.process.Child.run(.{
            .allocator = self.allocator,
            .argv = &[_][]const u8{ "sh", "-c", cmd },
        }) catch |err| {
            std.log.err("Failed to add IP {s} to nftables set: {}", .{ ip, err });
            return err;
        };
        defer {
            self.allocator.free(result.stdout);
            self.allocator.free(result.stderr);
        }

        if (result.term.Exited != 0) {
            // Don't error if IP already exists
            if (std.mem.indexOf(u8, result.stderr, "Object exists") != null) {
                std.log.debug("IP {s} already exists in nftables set", .{ip});
                return;
            }
            std.log.err("Failed to add IP {s} to nftables: {s}", .{ ip, result.stderr });
            return error.NftablesError;
        }

        std.log.debug("Added IP {s} to nftables set {s}", .{ ip, self.config.set_name });
    }

    pub fn removeIp(self: Self, ip: []const u8) !void {
        if (!self.isValidIp(ip)) {
            return error.InvalidIpAddress;
        }

        const cmd = try std.fmt.allocPrint(self.allocator, "nft delete element {s} {s} {s} {{ {s} }}", .{ 
            self.config.table_name, 
            self.config.table_name, 
            self.config.set_name, 
            ip 
        });
        defer self.allocator.free(cmd);

        const result = std.process.Child.run(.{
            .allocator = self.allocator,
            .argv = &[_][]const u8{ "sh", "-c", cmd },
        }) catch |err| {
            std.log.err("Failed to remove IP {s} from nftables set: {}", .{ ip, err });
            return err;
        };
        defer {
            self.allocator.free(result.stdout);
            self.allocator.free(result.stderr);
        }

        if (result.term.Exited != 0) {
            // Don't error if IP doesn't exist
            if (std.mem.indexOf(u8, result.stderr, "No such file or directory") != null) {
                std.log.debug("IP {s} not found in nftables set", .{ip});
                return;
            }
            std.log.err("Failed to remove IP {s} from nftables: {s}", .{ ip, result.stderr });
            return error.NftablesError;
        }

        std.log.debug("Removed IP {s} from nftables set {s}", .{ ip, self.config.set_name });
    }

    pub fn listIps(self: Self) ![][]const u8 {
        const cmd = try std.fmt.allocPrint(self.allocator, "nft list set {s} {s} {s} | grep -oE '([0-9]{{1,3}}\\.{{3}}[0-9]{{1,3}})'", .{ 
            self.config.table_name, 
            self.config.table_name, 
            self.config.set_name 
        });
        defer self.allocator.free(cmd);

        const result = std.process.Child.run(.{
            .allocator = self.allocator,
            .argv = &[_][]const u8{ "sh", "-c", cmd },
        }) catch |err| {
            std.log.err("Failed to list IPs from nftables set: {}", .{err});
            return err;
        };
        defer {
            self.allocator.free(result.stderr);
        }

        if (result.term.Exited != 0) {
            self.allocator.free(result.stdout);
            std.log.err("Failed to list nftables set contents", .{});
            return error.NftablesError;
        }

        var ips = std.ArrayList([]const u8).init(self.allocator);
        var lines = std.mem.splitSequence(u8, result.stdout, "\n");
        while (lines.next()) |line| {
            const trimmed = std.mem.trim(u8, line, " \t\r\n");
            if (trimmed.len > 0) {
                try ips.append(try self.allocator.dupe(u8, trimmed));
            }
        }

        self.allocator.free(result.stdout);
        return ips.toOwnedSlice();
    }

    pub fn flushSet(self: Self) !void {
        const cmd = try std.fmt.allocPrint(self.allocator, "nft flush set {s} {s} {s}", .{ 
            self.config.table_name, 
            self.config.table_name, 
            self.config.set_name 
        });
        defer self.allocator.free(cmd);

        const result = std.process.Child.run(.{
            .allocator = self.allocator,
            .argv = &[_][]const u8{ "sh", "-c", cmd },
        }) catch |err| {
            std.log.err("Failed to flush nftables set: {}", .{err});
            return err;
        };
        defer {
            self.allocator.free(result.stdout);
            self.allocator.free(result.stderr);
        }

        if (result.term.Exited != 0) {
            std.log.err("Failed to flush nftables set: {s}", .{result.stderr});
            return error.NftablesError;
        }

        std.log.info("Flushed nftables set {s}", .{self.config.set_name});
    }

    fn isValidIp(self: Self, ip: []const u8) bool {
        _ = self;
        // Basic IPv4 validation
        var parts = std.mem.splitScalar(u8, ip, '.');
        var part_count: u8 = 0;
        
        while (parts.next()) |part| {
            part_count += 1;
            if (part_count > 4) return false;
            
            const num = std.fmt.parseInt(u8, part, 10) catch return false;
            if (num > 255) return false;
        }
        
        return part_count == 4;
    }
};

test "nftables client init" {
    const config = Config.NftablesConfig{
        .set_name = "test-set",
        .table_name = "inet",
        .chain_name = "input",
    };

    // Skip actual nftables operations in test
    _ = config;
}

test "ip validation" {
    const config = Config.NftablesConfig{};
    const client = NftablesClient{
        .allocator = std.testing.allocator,
        .config = config,
    };

    try std.testing.expect(client.isValidIp("192.168.1.1"));
    try std.testing.expect(client.isValidIp("10.0.0.1"));
    try std.testing.expect(!client.isValidIp("256.1.1.1"));
    try std.testing.expect(!client.isValidIp("192.168.1"));
    try std.testing.expect(!client.isValidIp("not.an.ip.address"));
}
