use parking_lot::MappedRwLockReadGuard;
use prometheus_client::encoding::{EncodeMetric, MetricEncoder};
use prometheus_client::metrics::{family::Family, gauge, MetricType};

use std::fmt;
use std::hash::Hash;
use std::sync::atomic::AtomicU64;

/// Type alias for a Metric implementation for a float-64 prometheus Gauge.
/// Ironically, it was meant to provide more shorthand but now I have to implement all the important
/// traits just to delegate them to the wrapped inner type implementations...
pub(super) struct GaugeF<L>(Family<L, InnerFloat>);
type InnerFloat = gauge::Gauge<f64, AtomicU64>;

impl<L: Clone + Hash + Eq + PartialEq> GaugeF<L> {
    pub fn get_or_create(&self, label_set: &L) -> MappedRwLockReadGuard<InnerFloat> {
        self.0.get_or_create(label_set)
    }
    pub fn remove(&self, label_set: &L) -> bool {
        self.0.remove(label_set)
    }
    pub fn clear(&self) {
        self.0.clear()
    }
}

impl<L> Default for GaugeF<L>
where Family<L, InnerFloat>: Default
{
    fn default() -> Self {
        Self(Family::default())
    }
}

impl<L> Clone for GaugeF<L>
where Family<L, InnerFloat>: Clone
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<L> EncodeMetric for GaugeF<L>
where Family<L, InnerFloat>: EncodeMetric
{
    fn encode(&self, encoder: MetricEncoder<'_, '_>) -> fmt::Result {
        self.0.encode(encoder)
    }
    fn metric_type(&self) -> MetricType {
        self.0.metric_type()
    }
}

impl<L> fmt::Debug for GaugeF<L>
where Family<L, InnerFloat>: fmt::Debug
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
