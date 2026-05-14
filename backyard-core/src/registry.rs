use crate::{error::Result, job::JobContext};
use std::future::Future;
use std::pin::Pin;
use std::collections::HashMap;

pub struct JobRegistration {
    pub name: &'static str,
    pub handler: fn(
        raw: &[u8],
        ctx: JobContext,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>,
}

inventory::collect!(JobRegistration);

pub fn build_dispatch_table() -> HashMap<
    &'static str,
    fn(&[u8], JobContext) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>,
> {
    inventory::iter::<JobRegistration>()
        .map(|r| (r.name, r.handler))
        .collect()
}
