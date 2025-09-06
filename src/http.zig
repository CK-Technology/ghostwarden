const std = @import("std");
const Allocator = std.mem.Allocator;

pub const HttpClient = struct {
    allocator: Allocator,
    client: std.http.Client,

    const Self = @This();

    pub fn init(allocator: Allocator) Self {
        return Self{
            .allocator = allocator,
            .client = std.http.Client{ .allocator = allocator },
        };
    }

    pub fn deinit(self: *Self) void {
        self.client.deinit();
    }

    pub const AuthHeader = union(enum) {
        bearer: []const u8,
        basic: struct { username: []const u8, password: []const u8 },
        api_key: struct { key: []const u8, value: []const u8 },
        pve_token: struct { token_id: []const u8, token_secret: []const u8 },
    };

    pub const Request = struct {
        method: std.http.Method = .GET,
        url: []const u8,
        headers: ?std.StringHashMap([]const u8) = null,
        body: ?[]const u8 = null,
        auth: ?AuthHeader = null,
        verify_ssl: bool = true,
    };

    pub const Response = struct {
        status: std.http.Status,
        headers: std.StringHashMap([]const u8),
        body: []const u8,
        allocator: Allocator,

        pub fn deinit(self: *Response) void {
            self.allocator.free(self.body);
            var iter = self.headers.iterator();
            while (iter.next()) |entry| {
                self.allocator.free(entry.key_ptr.*);
                self.allocator.free(entry.value_ptr.*);
            }
            self.headers.deinit();
        }
    };

    pub fn request(self: *Self, req: Request) !Response {
        const uri = std.Uri.parse(req.url) catch |err| switch (err) {
            error.InvalidFormat => {
                std.log.err("Invalid URL format: {s}", .{req.url});
                return error.InvalidUrl;
            },
            else => return err,
        };

        var headers: std.ArrayList(std.http.Header) = .empty;
        defer headers.deinit(self.allocator);

        try headers.append(self.allocator, .{ .name = "user-agent", .value = "ghostwarden/0.1.0" });

        if (req.auth) |auth| {
            switch (auth) {
                .bearer => |token| {
                    const auth_value = try std.fmt.allocPrint(self.allocator, "Bearer {s}", .{token});
                    defer self.allocator.free(auth_value);
                    try headers.append(self.allocator, .{ .name = "authorization", .value = auth_value });
                },
                .basic => |creds| {
                    const auth_string = try std.fmt.allocPrint(self.allocator, "{s}:{s}", .{ creds.username, creds.password });
                    defer self.allocator.free(auth_string);
                    
                    var encoder = std.base64.standard.Encoder;
                    const encoded_len = encoder.calcSize(auth_string.len);
                    const encoded = try self.allocator.alloc(u8, encoded_len);
                    defer self.allocator.free(encoded);
                    _ = encoder.encode(encoded, auth_string);
                    
                    const auth_value = try std.fmt.allocPrint(self.allocator, "Basic {s}", .{encoded});
                    defer self.allocator.free(auth_value);
                    try headers.append(self.allocator, .{ .name = "authorization", .value = auth_value });
                },
                .api_key => |key| {
                    try headers.append(self.allocator, .{ .name = key.key, .value = key.value });
                },
                .pve_token => |token| {
                    const auth_value = try std.fmt.allocPrint(self.allocator, "PVEAPIToken={s}={s}", .{ token.token_id, token.token_secret });
                    defer self.allocator.free(auth_value);
                    try headers.append(self.allocator, .{ .name = "authorization", .value = auth_value });
                },
            }
        }

        if (req.headers) |custom_headers| {
            var iter = custom_headers.iterator();
            while (iter.next()) |entry| {
                try headers.append(self.allocator, .{ .name = entry.key_ptr.*, .value = entry.value_ptr.* });
            }
        }

        if (req.body != null) {
            try headers.append(self.allocator, .{ .name = "content-type", .value = "application/json" });
        }

        _ = try self.client.fetch(.{
            .method = req.method,
            .location = .{ .uri = uri },
            .extra_headers = headers.items,
            .payload = if (req.body) |body| body else null,
        });

        const response_headers = std.StringHashMap([]const u8).init(self.allocator);

        return Response{
            .status = .ok, // TODO: Get actual status from result
            .headers = response_headers,
            .body = "", // TODO: Get actual body from result  
            .allocator = self.allocator,
        };
    }

    pub fn get(self: *Self, url: []const u8, auth: ?AuthHeader) !Response {
        return self.request(.{ .url = url, .auth = auth });
    }

    pub fn post(self: *Self, url: []const u8, body: []const u8, auth: ?AuthHeader) !Response {
        return self.request(.{ .method = .POST, .url = url, .body = body, .auth = auth });
    }

    pub fn put(self: *Self, url: []const u8, body: []const u8, auth: ?AuthHeader) !Response {
        return self.request(.{ .method = .PUT, .url = url, .body = body, .auth = auth });
    }

    pub fn delete(self: *Self, url: []const u8, auth: ?AuthHeader) !Response {
        return self.request(.{ .method = .DELETE, .url = url, .auth = auth });
    }
};

test "http client init" {
    var client = HttpClient.init(std.testing.allocator);
    defer client.deinit();
}