use serde::Serialize;
use serde_json::{Map, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

const BATCH_CAP: usize = 1000;
#[derive(Debug, Clone)]
pub(crate) struct BatchItem {
    pub domain: String,
    pub command: String,
    pub parameters: Map<String, Value>,
    pub idempotency_key: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BatchItemResult {
    pub domain: String,
    pub command: String,
    pub idempotency_key: Option<String>,
    pub status: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BatchResult {
    pub batch_id: String,
    pub status: String,
    pub items: Vec<BatchItemResult>,
}

#[derive(Clone)]
pub(crate) struct BatchState {
    inner: Arc<Mutex<BatchStore>>,
    cap: usize,
}

#[derive(Default)]
struct BatchStore {
    results: HashMap<String, BatchResult>,
    order: VecDeque<String>,
}

impl BatchState {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BatchStore::default())),
            cap: BATCH_CAP,
        }
    }

    pub(crate) fn insert(&self, result: BatchResult) {
        let mut store = self.inner.lock().expect("batch state mutex poisoned");
        let batch_id = result.batch_id.clone();
        if store.results.contains_key(&batch_id) {
            store.order.retain(|id| id != &batch_id);
        }
        while store.results.len() >= self.cap {
            if let Some(oldest) = store.order.pop_front() {
                store.results.remove(&oldest);
            } else {
                break;
            }
        }
        store.order.push_back(batch_id.clone());
        store.results.insert(batch_id, result);
    }

    pub(crate) fn get(&self, batch_id: &str) -> Option<BatchResult> {
        self.inner
            .lock()
            .expect("batch state mutex poisoned")
            .results
            .get(batch_id)
            .cloned()
    }
}
