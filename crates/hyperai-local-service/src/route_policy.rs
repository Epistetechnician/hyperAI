//! Typed route definitions for the local service's scoped agent capability.

use axum::http::Method;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum AgentToolRouteMethod {
    Get,
    Post,
}

impl AgentToolRouteMethod {
    fn matches(self, method: &Method) -> bool {
        match self {
            Self::Get => *method == Method::GET,
            Self::Post => *method == Method::POST,
        }
    }

    #[cfg(test)]
    pub(crate) fn http_method(self) -> Method {
        match self {
            Self::Get => Method::GET,
            Self::Post => Method::POST,
        }
    }

    #[cfg(test)]
    fn opposite(self) -> Self {
        match self {
            Self::Get => Self::Post,
            Self::Post => Self::Get,
        }
    }

    #[cfg(test)]
    fn label(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum AgentToolRoute {
    DataCatalog,
    DataQuery,
    FleetInventory,
    FleetDoctor,
    FleetOperationPreview,
    EvalCatalog,
    EvalReports,
    WorkloadTrace,
    WorkloadTraceFailurePreview,
    WorkloadTraceInconclusivePreview,
    TradeCatalog,
    TradeSeries,
    TradeScenario,
    MarketSupplyComparison,
    MarketSupplyQuotePreview,
}

impl AgentToolRoute {
    pub(crate) const ALL: [Self; 15] = [
        Self::DataCatalog,
        Self::DataQuery,
        Self::FleetInventory,
        Self::FleetDoctor,
        Self::FleetOperationPreview,
        Self::EvalCatalog,
        Self::EvalReports,
        Self::WorkloadTrace,
        Self::WorkloadTraceFailurePreview,
        Self::WorkloadTraceInconclusivePreview,
        Self::TradeCatalog,
        Self::TradeSeries,
        Self::TradeScenario,
        Self::MarketSupplyComparison,
        Self::MarketSupplyQuotePreview,
    ];

    pub(crate) const fn method(self) -> AgentToolRouteMethod {
        match self {
            Self::DataQuery
            | Self::FleetOperationPreview
            | Self::TradeScenario
            | Self::MarketSupplyQuotePreview => AgentToolRouteMethod::Post,
            Self::DataCatalog
            | Self::FleetInventory
            | Self::FleetDoctor
            | Self::EvalCatalog
            | Self::EvalReports
            | Self::WorkloadTrace
            | Self::WorkloadTraceFailurePreview
            | Self::WorkloadTraceInconclusivePreview
            | Self::TradeCatalog
            | Self::TradeSeries
            | Self::MarketSupplyComparison => AgentToolRouteMethod::Get,
        }
    }

    pub(crate) const fn path(self) -> &'static str {
        match self {
            Self::DataCatalog => "/api/v1/agent/tools/data/catalog",
            Self::DataQuery => "/api/v1/agent/tools/data/query",
            Self::FleetInventory => "/api/v1/agent/tools/fleet/inventory",
            Self::FleetDoctor => "/api/v1/agent/tools/fleet/doctor",
            Self::FleetOperationPreview => "/api/v1/agent/tools/fleet/operation-preview",
            Self::EvalCatalog => "/api/v1/agent/tools/eval/catalog",
            Self::EvalReports => "/api/v1/agent/tools/eval/reports",
            Self::WorkloadTrace => "/api/v1/agent/tools/market/workload-trace",
            Self::WorkloadTraceFailurePreview => {
                "/api/v1/agent/tools/market/workload-trace/failure-preview"
            }
            Self::WorkloadTraceInconclusivePreview => {
                "/api/v1/agent/tools/market/workload-trace/inconclusive-preview"
            }
            Self::TradeCatalog => "/api/v1/agent/tools/trade/catalog",
            Self::TradeSeries => "/api/v1/agent/tools/trade/series/{instrument_id}",
            Self::TradeScenario => "/api/v1/agent/tools/trade/scenario",
            Self::MarketSupplyComparison => "/api/v1/agent/tools/market/supply-comparison",
            Self::MarketSupplyQuotePreview => "/api/v1/agent/tools/market/supply-quote-preview",
        }
    }

    fn matches(self, method: &Method, path: &str) -> bool {
        self.method().matches(method) && route_path_matches(self.path(), path)
    }
}

pub(crate) fn agent_tool_route_allowed(method: &Method, path: &str) -> bool {
    AgentToolRoute::ALL
        .iter()
        .any(|route| route.matches(method, path))
}

fn route_path_matches(pattern: &str, path: &str) -> bool {
    let mut pattern_segments = pattern.split('/');
    let mut path_segments = path.split('/');

    loop {
        match (pattern_segments.next(), path_segments.next()) {
            (None, None) => return true,
            (Some(pattern), Some(path_segment)) => {
                let is_parameter = pattern.starts_with('{') && pattern.ends_with('}');
                if is_parameter {
                    if path_segment.is_empty() {
                        return false;
                    }
                } else if pattern != path_segment {
                    return false;
                }
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use axum::http::Method;

    use super::{AgentToolRoute, agent_tool_route_allowed};

    #[test]
    fn frozen_capability_grant_is_exact() {
        let actual: Vec<_> = AgentToolRoute::ALL
            .iter()
            .map(|route| (route.method().label(), route.path()))
            .collect();
        let expected = [
            ("GET", "/api/v1/agent/tools/data/catalog"),
            ("POST", "/api/v1/agent/tools/data/query"),
            ("GET", "/api/v1/agent/tools/fleet/inventory"),
            ("GET", "/api/v1/agent/tools/fleet/doctor"),
            ("POST", "/api/v1/agent/tools/fleet/operation-preview"),
            ("GET", "/api/v1/agent/tools/eval/catalog"),
            ("GET", "/api/v1/agent/tools/eval/reports"),
            ("GET", "/api/v1/agent/tools/market/workload-trace"),
            (
                "GET",
                "/api/v1/agent/tools/market/workload-trace/failure-preview",
            ),
            (
                "GET",
                "/api/v1/agent/tools/market/workload-trace/inconclusive-preview",
            ),
            ("GET", "/api/v1/agent/tools/trade/catalog"),
            ("GET", "/api/v1/agent/tools/trade/series/{instrument_id}"),
            ("POST", "/api/v1/agent/tools/trade/scenario"),
            ("GET", "/api/v1/agent/tools/market/supply-comparison"),
            ("POST", "/api/v1/agent/tools/market/supply-quote-preview"),
        ];

        assert_eq!(actual, expected);

        let unique: HashSet<_> = AgentToolRoute::ALL
            .iter()
            .map(|route| (route.method(), route.path()))
            .collect();
        assert_eq!(unique.len(), AgentToolRoute::ALL.len());
    }

    #[test]
    fn declared_routes_allow_only_their_method_and_path() {
        for route in AgentToolRoute::ALL {
            let request_path = route.path().replace("{instrument_id}", "gpu-price-index");
            let method = route.method().http_method();
            assert!(
                agent_tool_route_allowed(&method, &request_path),
                "declared route denied: {method} {request_path}"
            );

            let wrong_method = route.method().opposite().http_method();
            assert!(
                !agent_tool_route_allowed(&wrong_method, &request_path),
                "wrong method allowed: {wrong_method} {request_path}"
            );
        }
    }

    #[test]
    fn parameter_route_rejects_missing_or_additional_segments() {
        assert!(!agent_tool_route_allowed(
            &Method::GET,
            "/api/v1/agent/tools/trade/series/"
        ));
        assert!(!agent_tool_route_allowed(
            &Method::GET,
            "/api/v1/agent/tools/trade/series/gpu-price-index/extra"
        ));
        assert!(!agent_tool_route_allowed(
            &Method::GET,
            "/api/v1/agent/tools/unknown"
        ));
        assert!(!agent_tool_route_allowed(
            &Method::GET,
            "/api/v1/data/catalog"
        ));
    }
}
