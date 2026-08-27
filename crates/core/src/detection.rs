use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, ToolMetadata};

pub struct DetectOptions<'a> {
    pub strict: bool,
    pub active_tool: Option<&'a str>,
    pub enabled: bool,
    pub cancel: &'a AtomicBool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recommendation {
    pub tool_id: String,
    pub data_type: String,
    pub payload: String,
}

struct DetectorNode {
    detector: Box<dyn Detector>,
    parent_name: Option<&'static str>,
    children: Vec<DetectorNode>,
}

struct Leaf {
    type_name: String,
    parent_name: Option<&'static str>,
    payload: String,
}

pub struct DetectionEngine {
    roots: Vec<DetectorNode>,
    tools_by_type: HashMap<String, Vec<String>>,
}

impl DetectionEngine {
    pub fn new(detectors: Vec<Box<dyn Detector>>, tools: &[ToolMetadata]) -> Self {
        Self {
            tools_by_type: map_tools(tools),
            roots: build_tree(detectors),
        }
    }

    pub fn detect(&self, raw: &RawData, options: DetectOptions<'_>) -> Vec<Recommendation> {
        if !options.enabled || options.cancel.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let leaves = walk(&self.roots, raw, None, options.cancel);
        if options.cancel.load(Ordering::Relaxed) {
            return Vec::new();
        }
        self.recommend(&leaves, options.strict, options.active_tool)
    }

    fn recommend(
        &self,
        leaves: &[Leaf],
        strict: bool,
        active_tool: Option<&str>,
    ) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        let skip = |id: &str| !strict && active_tool == Some(id);
        for leaf in leaves {
            if let Some(ids) = self.tools_by_type.get(&leaf.type_name) {
                for id in ids {
                    if skip(id) {
                        continue;
                    }
                    recs.push(Recommendation {
                        tool_id: id.clone(),
                        data_type: leaf.type_name.clone(),
                        payload: leaf.payload.clone(),
                    });
                }
            }
            if !strict {
                if let Some(parent) = leaf.parent_name {
                    if let Some(ids) = self.tools_by_type.get(parent) {
                        for id in ids {
                            if skip(id) {
                                continue;
                            }
                            recs.push(Recommendation {
                                tool_id: id.clone(),
                                data_type: parent.to_string(),
                                payload: leaf.payload.clone(),
                            });
                        }
                    }
                }
            }
        }
        recs
    }
}

fn map_tools(tools: &[ToolMetadata]) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for tool in tools {
        for ty in tool.accepted_types {
            if ty.is_empty() {
                continue;
            }
            map.entry((*ty).to_string())
                .or_default()
                .push(tool.id.as_str().to_string());
        }
    }
    map
}

fn build_tree(detectors: Vec<Box<dyn Detector>>) -> Vec<DetectorNode> {
    let n = detectors.len();
    if n == 0 {
        return Vec::new();
    }
    let specs: Vec<DataTypeSpec> = detectors.iter().map(|d| d.data_type()).collect();
    let mut name_index = HashMap::new();
    for (i, spec) in specs.iter().enumerate() {
        name_index.insert(spec.name, i);
    }
    let mut child_indices = vec![Vec::new(); n];
    let mut dropped_from_roots = vec![false; n];
    for (i, spec) in specs.iter().enumerate() {
        if let Some(parent_name) = spec.parent {
            dropped_from_roots[i] = true;
            if let Some(&parent) = name_index.get(parent_name) {
                child_indices[parent].push(i);
            }
        }
    }
    let mut slots: Vec<Option<Box<dyn Detector>>> = detectors.into_iter().map(Some).collect();
    (0..n)
        .filter(|&i| !dropped_from_roots[i])
        .map(|i| take_node(i, &mut slots, &child_indices, &specs))
        .collect()
}

fn take_node(
    i: usize,
    slots: &mut [Option<Box<dyn Detector>>],
    child_indices: &[Vec<usize>],
    specs: &[DataTypeSpec],
) -> DetectorNode {
    let detector = slots[i].take().expect("detector node taken twice");
    let children = child_indices[i]
        .iter()
        .copied()
        .map(|child| take_node(child, slots, child_indices, specs))
        .collect();
    DetectorNode {
        detector,
        parent_name: specs[i].parent,
        children,
    }
}

fn walk(
    nodes: &[DetectorNode],
    raw: &RawData,
    parent: Option<&DetectedPayload>,
    cancel: &AtomicBool,
) -> Vec<Leaf> {
    let mut leaves = Vec::new();
    for node in nodes {
        if cancel.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let Some(payload) = node.detector.detect(raw, parent) else {
            continue;
        };
        if node.children.is_empty() {
            leaves.push(leaf_from(node, payload));
            continue;
        }
        let nested = walk(&node.children, raw, Some(&payload), cancel);
        if cancel.load(Ordering::Relaxed) {
            return Vec::new();
        }
        if nested.is_empty() {
            leaves.push(leaf_from(node, payload));
        } else {
            leaves.extend(nested);
        }
    }
    leaves
}

fn leaf_from(node: &DetectorNode, payload: DetectedPayload) -> Leaf {
    Leaf {
        type_name: payload.type_name,
        parent_name: node.parent_name,
        payload: payload.value,
    }
}
