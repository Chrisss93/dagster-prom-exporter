use parking_lot::MappedRwLockReadGuard;
use prometheus_client::encoding::{EncodeMetric, MetricEncoder};
use prometheus_client::metrics::{family::Family, MetricType, gauge};

use core::fmt::{Debug, Formatter, Result};
use core::sync::atomic::AtomicU64;
use core::hash::Hash;

/// Type alias for a Metric implementation for a float-64 prometheus Gauge.
/// Ironically, it was meant to provide more shorthand but now I have to implement all the important
/// traits just to delegate them to the wrapped inner type implementations...
pub(super) struct GaugeF<L>(Family<L, InnerFloat>);
type InnerFloat = gauge::Gauge<f64, AtomicU64>;

impl <L: Clone + Hash + Eq + PartialEq> GaugeF<L> {
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


// impl <L: Clone + Hash + Eq> Default for GaugeF<L> {
//     fn default() -> Self {
//         Self(Default::default())
//     }
// }

impl <L> Default for GaugeF<L> where Family<L, InnerFloat>: Default {
    fn default() -> Self {
        Self(Family::default())
    }
}

impl <L> Clone for GaugeF<L> where Family<L, InnerFloat>: Clone {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl <L> EncodeMetric for GaugeF<L> where Family<L, InnerFloat>: EncodeMetric {
    fn encode(&self, encoder: MetricEncoder<'_, '_>) -> Result {
        self.0.encode(encoder)
    }
    fn metric_type(&self) -> MetricType {
        self.0.metric_type()
    }
}

impl <L> Debug for GaugeF<L> where Family<L, InnerFloat>: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.0.fmt(f)
    }
}
