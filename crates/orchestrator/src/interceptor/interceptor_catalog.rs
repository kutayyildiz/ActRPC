use actrpc_core::{
    InterceptorInitialization,
    action::{ActionDescriptor, ActionKind},
};

use crate::{
    endpoint::EndpointCatalog,
    error::{ActionExecutionError, InterceptorError, OrchestratorError},
    interceptor::{
        ImmutableInterceptorPipeline, Interceptor, InterceptorConfig, InterceptorPolicy,
        InterceptorRuntimeLimitsOverride, JsonRpcBackedInterceptor, WorkingInterceptorPipeline,
        initialization::validate_interceptor_registration,
    },
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone)]
pub struct InterceptorCatalogEntry {
    pub name: String,
    pub policy: InterceptorPolicy,
    pub interceptor: Arc<dyn Interceptor>,
    pub runtime_limits: Option<InterceptorRuntimeLimitsOverride>,
}

impl core::fmt::Debug for InterceptorCatalogEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InterceptorCatalogEntry")
            .field("name", &self.name)
            .field("policy", &self.policy)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct InterceptorCatalog {
    entries: HashMap<String, InterceptorCatalogEntry>,
    outbound_pipeline: ImmutableInterceptorPipeline,
    inbound_pipeline: ImmutableInterceptorPipeline,
}

impl InterceptorCatalog {
    pub fn new(
        entries: HashMap<String, InterceptorCatalogEntry>,
        outbound_pipeline: ImmutableInterceptorPipeline,
        inbound_pipeline: ImmutableInterceptorPipeline,
    ) -> Self {
        Self {
            entries,
            outbound_pipeline,
            inbound_pipeline,
        }
    }

    pub async fn build(
        interceptors: Vec<(InterceptorConfig, Arc<dyn Interceptor>)>,
        outbound_pipeline: Vec<String>,
        inbound_pipeline: Vec<String>,
        available_actions: &HashMap<ActionKind, ActionDescriptor>,
    ) -> Result<Self, OrchestratorError> {
        let mut entries = HashMap::new();
        let mut initializations = HashMap::new();

        for (config, interceptor) in interceptors {
            let initialization = interceptor.initialize().await.map_err(|source| {
                OrchestratorError::Interceptor(InterceptorError::InitializationFailed {
                    name: config.name.clone(),
                    source,
                })
            })?;

            validate_interceptor_registration(
                &config.name,
                &config.policy,
                &initialization,
                available_actions,
            )?;

            if entries.contains_key(&config.name) {
                return Err(OrchestratorError::Interceptor(
                    InterceptorError::DuplicateRegistration { name: config.name },
                ));
            }

            initializations.insert(config.name.clone(), initialization);

            entries.insert(
                config.name.clone(),
                InterceptorCatalogEntry {
                    name: config.name,
                    policy: config.policy,
                    interceptor,
                    runtime_limits: config.runtime,
                },
            );
        }

        validate_pipeline("outbound", &outbound_pipeline, &entries, &initializations)?;

        validate_pipeline("inbound", &inbound_pipeline, &entries, &initializations)?;

        Ok(Self::new(
            entries,
            ImmutableInterceptorPipeline::new(outbound_pipeline),
            ImmutableInterceptorPipeline::new(inbound_pipeline),
        ))
    }

    pub async fn build_from_configs(
        configs: Vec<InterceptorConfig>,
        outbound_pipeline: Vec<String>,
        inbound_pipeline: Vec<String>,
        available_actions: &HashMap<ActionKind, ActionDescriptor>,
        endpoint_catalog: &EndpointCatalog,
    ) -> Result<Self, OrchestratorError> {
        let mut interceptors = Vec::with_capacity(configs.len());

        for config in configs {
            if let Some(runtime) = config.runtime.as_ref() {
                runtime
                    .validate(&config.name)
                    .map_err(OrchestratorError::Config)?;
            }

            let endpoint = endpoint_catalog
                .get(&config.endpoint)
                .ok_or_else(|| {
                    OrchestratorError::Interceptor(InterceptorError::UnknownEndpoint {
                        name: config.endpoint.as_str().to_owned(),
                    })
                })?
                .clone();

            let interceptor: Arc<dyn Interceptor> =
                Arc::new(JsonRpcBackedInterceptor::new(endpoint));

            interceptors.push((config, interceptor));
        }

        Self::build(
            interceptors,
            outbound_pipeline,
            inbound_pipeline,
            available_actions,
        )
        .await
    }

    pub fn get_entry(&self, name: &str) -> Result<InterceptorCatalogEntry, ActionExecutionError> {
        let Some(entry) = self.entries.get(name) else {
            return Err(ActionExecutionError::NotFound {
                target: name.to_owned(),
            });
        };

        Ok(entry.clone())
    }

    pub fn entries(&self) -> Vec<InterceptorCatalogEntry> {
        self.entries.values().cloned().collect()
    }

    pub fn entries_for_names(
        &self,
        names: &[String],
    ) -> Result<Vec<InterceptorCatalogEntry>, ActionExecutionError> {
        let mut result = Vec::with_capacity(names.len());

        for name in names {
            let Some(entry) = self.entries.get(name) else {
                return Err(ActionExecutionError::NotFound {
                    target: name.clone(),
                });
            };

            result.push(entry.clone());
        }

        Ok(result)
    }

    pub fn outbound_pipeline_snapshot(&self) -> WorkingInterceptorPipeline {
        self.outbound_pipeline.snapshot()
    }

    pub fn inbound_pipeline_snapshot(&self) -> WorkingInterceptorPipeline {
        self.inbound_pipeline.snapshot()
    }
}

fn validate_pipeline(
    phase: &str,
    pipeline: &[String],
    entries: &HashMap<String, InterceptorCatalogEntry>,
    initializations: &HashMap<String, InterceptorInitialization>,
) -> Result<(), OrchestratorError> {
    let mut seen = HashSet::new();

    for name in pipeline {
        if !seen.insert(name.clone()) {
            return Err(OrchestratorError::Interceptor(
                InterceptorError::DuplicatePipelineEntry {
                    phase: phase.to_owned(),
                    name: name.clone(),
                },
            ));
        }

        let Some(initialization) = initializations.get(name) else {
            return Err(OrchestratorError::Interceptor(
                InterceptorError::UnknownPipelineInterceptor {
                    phase: phase.to_owned(),
                    name: name.clone(),
                },
            ));
        };

        if !entries.contains_key(name) {
            return Err(OrchestratorError::Interceptor(
                InterceptorError::UnknownPipelineInterceptor {
                    phase: phase.to_owned(),
                    name: name.clone(),
                },
            ));
        }

        match phase {
            "outbound" if !initialization.supports_outbound => {
                return Err(OrchestratorError::Interceptor(
                    InterceptorError::InvalidInitialization {
                        interceptor: name.clone(),
                        message: "interceptor is listed in outbound pipeline but does not support outbound"
                            .to_owned(),
                    },
                ));
            }
            "inbound" if !initialization.supports_inbound => {
                return Err(OrchestratorError::Interceptor(
                    InterceptorError::InvalidInitialization {
                        interceptor: name.clone(),
                        message:
                            "interceptor is listed in inbound pipeline but does not support inbound"
                                .to_owned(),
                    },
                ));
            }
            _ => {}
        }
    }

    Ok(())
}
