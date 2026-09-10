use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use devtoys_api::{DataTypeSpec, DetectedPayload, Detector, RawData, ToolMetadata, TYPE_IMAGE};

pub struct DetectOptions<'a> {
    pub strict: bool,
    /// Retained for API stability across call sites that construct this struct
    /// (integration tests). active_tool exclusion is no longer performed by the
    /// engine — the [`DetectionCoordinator`](crate::DetectionCoordinator) owns
    /// that projection, so this field is not read in the production path.
    pub active_tool: Option<&'a str>,
    pub enabled: bool,
    pub cancel: &'a AtomicBool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recommendation {
    pub tool_id: String,
    pub data_type: String,
    pub payload: String,
    /// Binary content from the detector (clipboard image bytes).
    pub bytes: Option<Vec<u8>>,
    /// MIME type accompanying [`Self::bytes`].
    pub mime: Option<String>,
}

impl Recommendation {
    pub fn new(
        tool_id: impl Into<String>,
        data_type: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            tool_id: tool_id.into(),
            data_type: data_type.into(),
            payload: payload.into(),
            bytes: None,
            mime: None,
        }
    }

    /// Payload the host should dispatch on recommendation click.
    ///
    /// Prefers bytes already on this recommendation. For `image` detections,
    /// falls back to the original clipboard `RawData::Image` so a MIME string
    /// is never used as a file path.
    pub fn paste_payload(&self, clipboard: Option<&RawData>) -> DetectedPayload {
        if let Some(bytes) = self.bytes.as_ref().filter(|b| !b.is_empty()) {
            return DetectedPayload::new(self.data_type.clone(), self.payload.clone())
                .with_bytes(bytes.clone(), self.mime.clone());
        }
        if self.data_type == TYPE_IMAGE {
            if let Some(RawData::Image { bytes, mime }) = clipboard {
                if !bytes.is_empty() {
                    return DetectedPayload::new(self.data_type.clone(), self.payload.clone())
                        .with_bytes(bytes.clone(), mime.clone());
                }
            }
        }
        DetectedPayload::new(self.data_type.clone(), self.payload.clone())
    }
}

/// Registration errors surfaced when assembling the detector tree plus tool
/// metadata. Structural issues (`DuplicateType`, `MissingParent`) indicate the
/// tree cannot be built correctly; `UnresolvedToolType` is an audit finding that
/// does not prevent the engine from running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetectionAssemblyIssue {
    /// More than one detector declares the same type name.
    DuplicateType { type_name: String },
    /// A detector's declared parent type is not present in the detector set.
    MissingParent {
        type_name: String,
        parent_name: String,
    },
    /// A tool declares an accepted type that no detector produces.
    UnresolvedToolType { tool_id: String, type_name: String },
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
    bytes: Option<Vec<u8>>,
    mime: Option<String>,
}

pub struct DetectionEngine {
    roots: Vec<DetectorNode>,
    tools_by_type: HashMap<String, Vec<String>>,
}

impl DetectionEngine {
    pub fn new(detectors: Vec<Box<dyn Detector>>, tools: &[ToolMetadata]) -> Self {
        let issues = validate(&detectors, tools);
        // Surface structural registration errors in development; `UnresolvedToolType`
        // is an audit finding and does not block engine construction.
        debug_assert!(
            issues
                .iter()
                .all(|i| matches!(i, DetectionAssemblyIssue::UnresolvedToolType { .. })),
            "Detection assembly issues: {issues:?}"
        );
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
        self.recommend(&leaves, options.strict)
    }

    /// Builds recommendations from the detected leaves. This is a pure
    /// projection over the detector tree: it never excludes the active tool —
    /// active_tool filtering is owned by the
    /// [`DetectionCoordinator`](crate::DetectionCoordinator) so that leaving a
    /// tool page can restore the recommendation from cached raw hits.
    fn recommend(&self, leaves: &[Leaf], strict: bool) -> Vec<Recommendation> {
        let mut recs = Vec::new();
        for leaf in leaves {
            if let Some(ids) = self.tools_by_type.get(&leaf.type_name) {
                for id in ids {
                    recs.push(Recommendation {
                        tool_id: id.clone(),
                        data_type: leaf.type_name.clone(),
                        payload: leaf.payload.clone(),
                        bytes: leaf.bytes.clone(),
                        mime: leaf.mime.clone(),
                    });
                }
            }
            if !strict {
                if let Some(parent) = leaf.parent_name {
                    if let Some(ids) = self.tools_by_type.get(parent) {
                        for id in ids {
                            recs.push(Recommendation {
                                tool_id: id.clone(),
                                data_type: parent.to_string(),
                                payload: leaf.payload.clone(),
                                bytes: leaf.bytes.clone(),
                                mime: leaf.mime.clone(),
                            });
                        }
                    }
                }
            }
        }
        recs
    }
}

/// Validates the assembly of a detector set against the tool metadata.
///
/// Reports every type name registered by more than one detector
/// ([`DetectionAssemblyIssue::DuplicateType`]), every detector whose declared
/// parent is absent from the set ([`DetectionAssemblyIssue::MissingParent`]),
/// and every tool accepted type that no detector produces
/// ([`DetectionAssemblyIssue::UnresolvedToolType`]).
pub fn validate(
    detectors: &[Box<dyn Detector>],
    tools: &[ToolMetadata],
) -> Vec<DetectionAssemblyIssue> {
    let mut issues = Vec::new();

    let mut name_counts: HashMap<&'static str, usize> = HashMap::new();
    for detector in detectors {
        *name_counts.entry(detector.data_type().name).or_default() += 1;
    }

    let mut reported = HashSet::new();
    for detector in detectors {
        let name = detector.data_type().name;
        if name_counts.get(name).copied().unwrap_or(0) > 1 && reported.insert(name) {
            issues.push(DetectionAssemblyIssue::DuplicateType {
                type_name: name.to_string(),
            });
        }
    }

    for detector in detectors {
        let spec = detector.data_type();
        if let Some(parent) = spec.parent {
            if !name_counts.contains_key(parent) {
                issues.push(DetectionAssemblyIssue::MissingParent {
                    type_name: spec.name.to_string(),
                    parent_name: parent.to_string(),
                });
            }
        }
    }

    for tool in tools {
        for ty in tool.accepted_types {
            if !name_counts.contains_key(ty) {
                issues.push(DetectionAssemblyIssue::UnresolvedToolType {
                    tool_id: tool.id.as_str().to_string(),
                    type_name: ty.to_string(),
                });
            }
        }
    }

    issues
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

/// Builds the root detector nodes, dropping nodes whose declared parent type is
/// not present. Prefer calling [`validate`] to surface such registration errors;
/// the tree still assembles so the engine can run.
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
        bytes: payload.bytes,
        mime: payload.mime,
    }
}
