const std = @import("std");
const Allocator = std.mem.Allocator;
const Http = @import("http.zig");
const Config = @import("config.zig");

pub const Decision = struct {
    id: i64,
    origin: []const u8,
    type: []const u8,
    scope: []const u8,
    value: []const u8,
    duration: []const u8,
    scenario: []const u8,
    simulated: bool,
    created_at: []const u8,
    updated_at: []const u8,
};

pub const DecisionResponse = struct {
    new: ?[]Decision = null,
    deleted: ?[]Decision = null,
};

pub const CrowdSecClient = struct {
    allocator: Allocator,
    config: Config.CrowdSecConfig,
    http_client: Http.HttpClient,
    machine_id: []const u8,

    const Self = @This();

    pub fn init(allocator: Allocator, config: Config.CrowdSecConfig) !Self {
        const machine_id = config.machine_id orelse blk: {
            var uuid_bytes: [16]u8 = undefined;
            std.crypto.random.bytes(uuid_bytes[0..]);
            
            const uuid_str = try std.fmt.allocPrint(allocator, "{x}-{x}-{x}-{x}-{x}", .{
                std.mem.readInt(u32, uuid_bytes[0..4], .big),
                std.mem.readInt(u16, uuid_bytes[4..6], .big),
                std.mem.readInt(u16, uuid_bytes[6..8], .big),
                std.mem.readInt(u16, uuid_bytes[8..10], .big),
                std.mem.readInt(u48, uuid_bytes[10..16], .big),
            });
            break :blk uuid_str;
        };

        return Self{
            .allocator = allocator,
            .config = config,
            .http_client = Http.HttpClient.init(allocator),
            .machine_id = machine_id,
        };
    }

    pub fn deinit(self: *Self) void {
        if (self.config.machine_id == null) {
            self.allocator.free(self.machine_id);
        }
        self.http_client.deinit();
    }

    pub fn getDecisions(self: *Self, startup: bool) !DecisionResponse {
        const url_path = if (startup) "/v1/decisions/stream?startup=true" else "/v1/decisions/stream";
        const url = try std.fmt.allocPrint(self.allocator, "{s}{s}", .{ self.config.lapi_url, url_path });
        defer self.allocator.free(url);

        const auth = Http.HttpClient.AuthHeader{ .api_key = .{ .key = "X-Api-Key", .value = self.config.api_key } };
        var response = self.http_client.get(url, auth) catch |err| {
            std.log.err("Failed to connect to CrowdSec LAPI at {s}: {}", .{ self.config.lapi_url, err });
            return err;
        };
        defer response.deinit();

        switch (response.status) {
            .ok => {},
            .unauthorized => {
                std.log.err("CrowdSec LAPI authentication failed. Check your API key.", .{});
                return error.AuthenticationError;
            },
            .not_found => {
                std.log.err("CrowdSec LAPI endpoint not found. Check your LAPI URL.", .{});
                return error.ApiError;
            },
            else => {
                std.log.err("CrowdSec LAPI returned status: {}", .{response.status});
                return error.ApiError;
            },
        }

        return self.parseDecisionResponse(response.body);
    }

    fn parseDecisionResponse(self: *Self, body: []const u8) !DecisionResponse {
        const parsed = std.json.parseFromSlice(std.json.Value, self.allocator, body, .{}) catch |err| switch (err) {
            error.SyntaxError => {
                std.log.err("Invalid JSON response from CrowdSec LAPI", .{});
                return error.ParseError;
            },
            else => return err,
        };
        defer parsed.deinit();

        const root = parsed.value;
        var result = DecisionResponse{};

        if (root.object.get("new")) |new_decisions| {
            if (new_decisions == .array) {
                result.new = try self.parseDecisions(new_decisions.array.items);
            }
        }

        if (root.object.get("deleted")) |deleted_decisions| {
            if (deleted_decisions == .array) {
                result.deleted = try self.parseDecisions(deleted_decisions.array.items);
            }
        }

        return result;
    }

    fn parseDecisions(self: *Self, decisions: []std.json.Value) ![]Decision {
        var result: std.ArrayList(Decision) = .empty;
        try result.ensureTotalCapacity(self.allocator, decisions.len);
        defer result.deinit(self.allocator);

        for (decisions) |decision_value| {
            if (decision_value != .object) continue;
            const obj = decision_value.object;

            const decision = Decision{
                .id = if (obj.get("id")) |v| v.integer else 0,
                .origin = if (obj.get("origin")) |v| try self.allocator.dupe(u8, v.string) else "",
                .type = if (obj.get("type")) |v| try self.allocator.dupe(u8, v.string) else "ban",
                .scope = if (obj.get("scope")) |v| try self.allocator.dupe(u8, v.string) else "Ip",
                .value = if (obj.get("value")) |v| try self.allocator.dupe(u8, v.string) else "",
                .duration = if (obj.get("duration")) |v| try self.allocator.dupe(u8, v.string) else "4h",
                .scenario = if (obj.get("scenario")) |v| try self.allocator.dupe(u8, v.string) else "",
                .simulated = if (obj.get("simulated")) |v| v.bool else false,
                .created_at = if (obj.get("created_at")) |v| try self.allocator.dupe(u8, v.string) else "",
                .updated_at = if (obj.get("updated_at")) |v| try self.allocator.dupe(u8, v.string) else "",
            };

            try result.append(self.allocator, decision);
        }

        return result.toOwnedSlice(self.allocator);
    }

    pub fn heartbeat(self: *Self) !void {
        const url = try std.fmt.allocPrint(self.allocator, "{s}/v1/heartbeat", .{self.config.lapi_url});
        defer self.allocator.free(url);

        const body = try std.fmt.allocPrint(self.allocator, "{{\"machine_id\":\"{s}\"}}", .{self.machine_id});
        defer self.allocator.free(body);

        const auth = Http.HttpClient.AuthHeader{ .api_key = .{ .key = "X-Api-Key", .value = self.config.api_key } };
        var response = self.http_client.post(url, body, auth) catch |err| {
            std.log.warn("CrowdSec heartbeat failed: {}", .{err});
            return;
        };
        defer response.deinit();

        switch (response.status) {
            .ok => std.log.debug("CrowdSec heartbeat successful", .{}),
            else => std.log.warn("CrowdSec heartbeat failed with status: {}", .{response.status}),
        }
    }

    pub fn validateConfig(config: Config.CrowdSecConfig) !void {
        if (config.lapi_url.len == 0) {
            return error.ConfigurationError;
        }
        if (config.api_key.len == 0) {
            return error.ConfigurationError;
        }
        if (config.poll_interval_seconds < 10) {
            return error.ConfigurationError;
        }
    }
};

test "crowdsec client init" {
    const config = Config.CrowdSecConfig{
        .lapi_url = "http://localhost:8080",
        .api_key = "test-key",
    };

    var client = try CrowdSecClient.init(std.testing.allocator, config);
    defer client.deinit();
}