use std::collections::HashMap;
use std::iter::FromIterator;
use std::time::Instant;

use async_trait::async_trait;

use graph::components::store::StoredDynamicDataSource;
use graph::prelude::{DataSourceLoader as DataSourceLoaderTrait, *};

pub struct DataSourceLoader<S> {
    store: Arc<S>,
}

impl<S> DataSourceLoader<S>
where
    S: Store,
{
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S> DataSourceLoaderTrait for DataSourceLoader<S>
where
    S: Store,
{
    async fn load_dynamic_data_sources(
        &self,
        deployment_id: SubgraphDeploymentId,
        logger: Logger,
        manifest: SubgraphManifest,
    ) -> Result<Vec<DataSource>, Error> {
        let start_time = Instant::now();

        let template_map: HashMap<&str, &DataSourceTemplate> = HashMap::from_iter(
            manifest
                .templates
                .iter()
                .map(|template| (template.name.as_str(), template)),
        );
        let mut data_sources: Vec<DataSource> = vec![];

        for stored in self.store.load_dynamic_data_sources(&deployment_id)? {
            let StoredDynamicDataSource {
                name,
                source,
                context,
                creation_block,
            } = stored;

            let template = template_map.get(name.as_str()).ok_or_else(|| {
                format_err!(
                    "deployment `{}` does not have a template called `{}`",
                    deployment_id.as_str(),
                    name
                )
            })?;
            let context = context
                .map(|ctx| serde_json::from_str::<Entity>(&ctx))
                .transpose()?;

            let ds = DataSource {
                kind: template.kind.clone(),
                network: template.network.clone(),
                name,
                source,
                mapping: template.mapping.clone(),
                context,
                creation_block,
                templates: Vec::new(),
            };

            // The data sources are ordered by the creation block.
            // See also 8f1bca33-d3b7-4035-affc-fd6161a12448.
            assert!(data_sources.last().and_then(|d| d.creation_block) <= ds.creation_block);

            data_sources.push(ds);
        }

        trace!(
            logger,
            "Loaded dynamic data sources";
            "ms" => start_time.elapsed().as_millis()
        );

        Ok(data_sources)
    }
}
