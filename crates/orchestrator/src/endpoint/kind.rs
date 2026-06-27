#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointKind {
    JsonRpc2,
    RestHttp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndpointCapabilities {
    pub request_response: bool,
    pub persistent: bool,
    pub notifications: bool,
}

impl EndpointCapabilities {
    pub const REQUEST_RESPONSE: Self = Self {
        request_response: true,
        persistent: false,
        notifications: false,
    };

    pub const SESSION: Self = Self {
        request_response: true,
        persistent: true,
        notifications: true,
    };

    pub const REST_HTTP: Self = Self {
        request_response: true,
        persistent: false,
        notifications: false,
    };
}
