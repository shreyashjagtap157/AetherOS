//! Merkle DAG integrity reference model (RFC-0017).
//!
//! Every persistent object is part of a Merkle Directed Acyclic Graph (DAG).
//! The root of the DAG is the object identity (RFC-0015). Leaf objects hash
//! their content bytes; composite objects hash their metadata, ordered child
//! identities, and a structure-type marker. Integrity verification confirms
//! that content matches its declared identity before returning data to a
//! consumer (RFC-0017.5). Only the path from a changed child to the root is
//! recomputed on update (RFC-0017.7).

use aether_object_identity::ObjectIdentity;
use std::collections::{HashMap, HashSet};

/// Maximum Merkle tree depth supported by the platform (RFC-0017.4: at least 64).
pub const MAX_DEPTH: usize = 64;

/// Node type in the Merkle DAG (RFC-0017.3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NodeType {
    /// Leaf object: identity is the hash of its content bytes (RFC-0017.2).
    Leaf = 0,
    /// Internal object: identity is derived from metadata and child identities (RFC-0017.3).
    Internal = 1,
    /// Directory object: identity is derived from metadata and child identities (RFC-0017.3).
    Directory = 2,
}

/// Integrity verification outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegrityResult {
    /// Content matches its declared identity.
    Valid,
    /// Content does not match its declared identity.
    Invalid,
}

/// Integrity fault types (RFC-0017.6).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntegrityFault {
    /// Object content does not match its declared identity.
    ContentMismatch,
    /// Signature verification failed (RFC-0017.9).
    SignatureInvalid,
    /// A Merkle path is broken (missing node or inconsistent hash).
    MerklePathBroken,
}

/// A node in the Merkle DAG (RFC-0017.3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MerkleNode {
    /// BLAKE3-256 hash of this node's content (leaf) or composite representation.
    pub hash: ObjectIdentity,
    /// Type of node: leaf, internal, or directory.
    pub node_type: NodeType,
    /// Number of child nodes (0 for leaf).
    pub child_count: usize,
    /// Ordered list of child object identities.
    pub children: Vec<ObjectIdentity>,
    /// Bytes of actual content at this node.
    pub content_size: u64,
    /// Metadata bytes used in composite identity computation.
    pub metadata: Vec<u8>,
    /// Structure-type marker used in composite identity computation.
    pub structure_marker: u8,
}

/// A Merkle DAG with integrity verification support (RFC-0017.1).
pub struct MerkleDAG {
    /// Root identity of the DAG — the object identity (RFC-0017.1).
    pub root: ObjectIdentity,
    /// All nodes in the DAG, keyed by their identity.
    pub nodes: HashMap<ObjectIdentity, MerkleNode>,
    /// Reverse index: child identity -> parent identities that reference it.
    pub parents: HashMap<ObjectIdentity, Vec<ObjectIdentity>>,
    /// Maximum allowed Merkle tree depth (RFC-0017.4).
    max_depth: usize,
}

/// Signed integrity for a Merkle DAG root (RFC-0017.8).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedIntegrity {
    /// Root object identity of the signed Merkle DAG.
    pub object_id: ObjectIdentity,
    /// Identifier of the signing key.
    pub key_identifier: Vec<u8>,
    /// Signature over the root identity (64 bytes, Ed25519 or equivalent).
    pub signature: [u8; 64],
    /// Declared signature algorithm identifier.
    pub signature_algorithm: u8,
}

impl MerkleDAG {
    /// Creates a new [`MerkleDAG`] with the given root identity.
    #[must_use]
    pub fn new(root: ObjectIdentity) -> Self {
        Self {
            root,
            nodes: HashMap::new(),
            parents: HashMap::new(),
            max_depth: MAX_DEPTH,
        }
    }

    /// Inserts a leaf node into the DAG (RFC-0017.2).
    /// The node's identity is BLAKE3-256 of its content bytes.
    pub fn insert_leaf(&mut self, hash: ObjectIdentity, content_size: u64) {
        let node = MerkleNode {
            hash,
            node_type: NodeType::Leaf,
            child_count: 0,
            children: Vec::new(),
            content_size,
            metadata: Vec::new(),
            structure_marker: 0,
        };
        self.nodes.insert(hash, node);
    }

    /// Inserts a composite node into the DAG (RFC-0017.3).
    /// Parent links are recorded for incremental verification (RFC-0017.7).
    pub fn insert_composite(
        &mut self,
        hash: ObjectIdentity,
        node_type: NodeType,
        children: &[ObjectIdentity],
        metadata: Vec<u8>,
        structure_marker: u8,
        content_size: u64,
    ) {
        let child_count = children.len();
        let node = MerkleNode {
            hash,
            node_type,
            child_count,
            children: children.to_owned(),
            content_size,
            metadata,
            structure_marker,
        };
        self.nodes.insert(hash, node);
        for child in children {
            self.parents.entry(*child).or_default().push(hash);
        }
    }

    /// Computes the identity of a leaf node from its content bytes (RFC-0017.2).
    #[must_use]
    pub fn leaf_identity(content: &[u8]) -> ObjectIdentity {
        ObjectIdentity::compute(content)
    }

    /// Computes the identity of a composite node from metadata, ordered child
    /// identities, and a structure-type marker (RFC-0017.3).
    #[must_use]
    pub fn composite_identity(
        metadata: &[u8],
        children: &[ObjectIdentity],
        structure_marker: u8,
    ) -> ObjectIdentity {
        let mut buf = Vec::with_capacity(metadata.len() + children.len() * 32 + 1);
        buf.extend_from_slice(metadata);
        for child in children {
            buf.extend_from_slice(child.as_ref());
        }
        buf.push(structure_marker);
        ObjectIdentity::compute(&buf)
    }

    /// Returns the depth of the node identified by `identity` in the DAG.
    /// Returns `None` if the node is not present or the depth exceeds `max_depth`.
    fn node_depth(
        &self,
        identity: &ObjectIdentity,
        visited: &mut HashSet<ObjectIdentity>,
    ) -> Option<usize> {
        if visited.len() > self.max_depth {
            return None;
        }
        if !visited.insert(*identity) {
            return None; // cycle — not a DAG
        }
        let node = self.nodes.get(identity)?;
        if node.child_count == 0 {
            return Some(0);
        }
        let mut max_child_depth = 0usize;
        for child in &node.children {
            let child_depth = self.node_depth(child, visited)?;
            max_child_depth = max_child_depth.max(child_depth + 1);
        }
        visited.remove(identity);
        Some(max_child_depth)
    }

    /// Verifies that the DAG depth does not exceed the maximum (RFC-0017.4).
    ///
    /// # Errors
    ///
    /// Returns [`IntegrityFault::MerklePathBroken`] if the root is missing or
    /// the tree depth exceeds `max_depth`.
    pub fn verify_depth(&self) -> Result<(), IntegrityFault> {
        let depth = self
            .node_depth(&self.root, &mut HashSet::new())
            .ok_or(IntegrityFault::MerklePathBroken)?;
        if depth > self.max_depth {
            return Err(IntegrityFault::MerklePathBroken);
        }
        Ok(())
    }

    /// Recomputes the root identity along the path from `changed` to the root,
    /// using `new_identity` as the updated identity of `changed`. Only the
    /// affected path is recomputed; sibling subtrees are not traversed
    /// (RFC-0017.7 incremental verification).
    ///
    /// # Errors
    ///
    /// Returns [`IntegrityFault::MerklePathBroken`] if a node on the path to
    /// the root is missing from the DAG.
    pub fn recompute_root(
        &self,
        changed: &ObjectIdentity,
        new_identity: &ObjectIdentity,
    ) -> Result<ObjectIdentity, IntegrityFault> {
        let mut current = *changed;
        let mut current_new = *new_identity;

        loop {
            let parents = self.parents.get(&current);
            match parents {
                None => return Ok(current_new),
                Some(parent_list) => {
                    let parent = parent_list[0];
                    let parent_node = self
                        .nodes
                        .get(&parent)
                        .ok_or(IntegrityFault::MerklePathBroken)?;
                    // Build the updated children list with `current` replaced by `current_new`.
                    let mut new_children = parent_node.children.clone();
                    for child in &mut new_children {
                        if *child == current {
                            *child = current_new;
                        }
                    }
                    let marker = match parent_node.node_type {
                        NodeType::Internal => 1,
                        NodeType::Directory => 2,
                        NodeType::Leaf => 0,
                    };
                    current_new =
                        Self::composite_identity(&parent_node.metadata, &new_children, marker);
                    current = parent;
                }
            }
        }
    }
}

/// Integrity verifier for a [`MerkleDAG`] (RFC-0017.5, RFC-0017.6).
pub struct IntegrityVerifier<'a> {
    dag: &'a MerkleDAG,
}

impl<'a> IntegrityVerifier<'a> {
    /// Creates a new integrity verifier for the given DAG (RFC-0017.5).
    #[must_use]
    pub fn new(dag: &'a MerkleDAG) -> Self {
        Self { dag }
    }

    /// Verifies that a leaf node's content matches its declared identity
    /// (RFC-0017.5). Returns [`IntegrityResult::Invalid`] on mismatch
    /// (RFC-0017.6).
    ///
    /// # Errors
    ///
    /// Returns [`IntegrityFault::MerklePathBroken`] if the identity is not
    /// present in the DAG.
    pub fn verify_leaf(
        &self,
        identity: &ObjectIdentity,
        content: &[u8],
    ) -> Result<IntegrityResult, IntegrityFault> {
        let node = self
            .dag
            .nodes
            .get(identity)
            .ok_or(IntegrityFault::MerklePathBroken)?;
        if node.node_type != NodeType::Leaf {
            return Ok(IntegrityResult::Invalid);
        }
        let expected = MerkleDAG::leaf_identity(content);
        if expected == *identity {
            Ok(IntegrityResult::Valid)
        } else {
            Ok(IntegrityResult::Invalid)
        }
    }

    /// Verifies that a composite node's identity matches its metadata,
    /// children, and structure marker (RFC-0017.5). Returns
    /// [`IntegrityResult::Invalid`] on mismatch (RFC-0017.6).
    ///
    /// # Errors
    ///
    /// Returns [`IntegrityFault::MerklePathBroken`] if the identity is not
    /// present in the DAG.
    pub fn verify_composite(
        &self,
        identity: &ObjectIdentity,
        metadata: &[u8],
        structure_marker: u8,
    ) -> Result<IntegrityResult, IntegrityFault> {
        let node = self
            .dag
            .nodes
            .get(identity)
            .ok_or(IntegrityFault::MerklePathBroken)?;
        if matches!(node.node_type, NodeType::Leaf) {
            return Ok(IntegrityResult::Invalid);
        }
        if node.children.len() != node.child_count {
            return Ok(IntegrityResult::Invalid);
        }
        let expected = MerkleDAG::composite_identity(metadata, &node.children, structure_marker);
        if expected == *identity {
            Ok(IntegrityResult::Valid)
        } else {
            Ok(IntegrityResult::Invalid)
        }
    }

    /// Verifies the path from `identity` up to the root (RFC-0017.7
    /// incremental verification). Only the affected path is checked.
    ///
    /// # Errors
    ///
    /// Returns [`IntegrityFault::MerklePathBroken`] if a node on the path is
    /// missing from the DAG.
    pub fn verify_path(
        &self,
        identity: &ObjectIdentity,
    ) -> Result<IntegrityResult, IntegrityFault> {
        let mut current = *identity;
        loop {
            let node = self
                .dag
                .nodes
                .get(&current)
                .ok_or(IntegrityFault::MerklePathBroken)?;
            match node.node_type {
                NodeType::Leaf => {
                    // Leaf verification requires content; without it, accept the
                    // declared hash as structurally consistent.
                }
                NodeType::Internal | NodeType::Directory => {
                    if node.children.len() != node.child_count {
                        return Ok(IntegrityResult::Invalid);
                    }
                }
            }
            let parents = self.dag.parents.get(&current);
            match parents {
                None => return Ok(IntegrityResult::Valid),
                Some(list) => current = list[0],
            }
        }
    }
}

impl SignedIntegrity {
    /// Creates a new signed integrity declaration (RFC-0017.8).
    #[must_use]
    pub fn new(
        object_id: ObjectIdentity,
        key_identifier: Vec<u8>,
        signature: [u8; 64],
        signature_algorithm: u8,
    ) -> Self {
        Self {
            object_id,
            key_identifier,
            signature,
            signature_algorithm,
        }
    }

    /// Verifies the signature. In this reference model, verification is a
    /// structural check that the signature is non-zero and the algorithm is
    /// declared. Real cryptographic verification is delegated to the signing
    /// subsystem (RFC-0017.8: signature verification is independent of storage
    /// provider).
    #[must_use]
    pub fn verify(&self) -> IntegrityResult {
        if self.signature == [0u8; 64] || self.key_identifier.is_empty() {
            IntegrityResult::Invalid
        } else {
            IntegrityResult::Valid
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_id(bytes: &[u8]) -> ObjectIdentity {
        ObjectIdentity::compute(bytes)
    }

    #[test]
    fn leaf_identity_is_hash_of_content() {
        let content = b"leaf object content";
        let id = MerkleDAG::leaf_identity(content);
        let expected = ObjectIdentity::compute(content);
        assert_eq!(id, expected);
    }

    #[test]
    fn composite_identity_derives_from_metadata_children_marker() {
        let c1 = make_id(b"child one");
        let c2 = make_id(b"child two");
        let id = MerkleDAG::composite_identity(b"meta", &[c1, c2], 1);
        // Recomputing with the same inputs yields the same identity.
        let id2 = MerkleDAG::composite_identity(b"meta", &[c1, c2], 1);
        assert_eq!(id, id2);
        // Different metadata or children yields a different identity.
        let id3 = MerkleDAG::composite_identity(b"other", &[c1, c2], 1);
        assert_ne!(id, id3);
        let id4 = MerkleDAG::composite_identity(b"meta", &[c1], 1);
        assert_ne!(id, id4);
    }

    #[test]
    fn dag_maintains_merkle_structure() {
        let mut dag = MerkleDAG::new(ObjectIdentity::compute(b"root"));
        let leaf = MerkleDAG::leaf_identity(b"leaf content");
        dag.insert_leaf(leaf, 13);
        assert!(dag.nodes.contains_key(&leaf));
    }

    #[test]
    fn verify_leaf_matching_content_is_valid() {
        let mut dag = MerkleDAG::new(ObjectIdentity::compute(b"root"));
        let content = b"verifiable leaf content";
        let id = MerkleDAG::leaf_identity(content);
        dag.insert_leaf(id, content.len() as u64);
        let verifier = IntegrityVerifier::new(&dag);
        assert_eq!(
            verifier.verify_leaf(&id, content).unwrap(),
            IntegrityResult::Valid
        );
    }

    #[test]
    fn verify_leaf_tampered_content_is_invalid() {
        let mut dag = MerkleDAG::new(ObjectIdentity::compute(b"root"));
        let content = b"original content";
        let id = MerkleDAG::leaf_identity(content);
        dag.insert_leaf(id, content.len() as u64);
        let verifier = IntegrityVerifier::new(&dag);
        assert_eq!(
            verifier.verify_leaf(&id, b"tampered content").unwrap(),
            IntegrityResult::Invalid
        );
    }

    #[test]
    fn verify_composite_matching_is_valid() {
        let mut dag = MerkleDAG::new(ObjectIdentity::compute(b"root"));
        let c1 = make_id(b"child one");
        let c2 = make_id(b"child two");
        let meta = b"composite meta";
        let id = MerkleDAG::composite_identity(meta, &[c1, c2], 1);
        dag.insert_composite(id, NodeType::Internal, &[c1, c2], meta.to_vec(), 1, 0);
        let verifier = IntegrityVerifier::new(&dag);
        assert_eq!(
            verifier.verify_composite(&id, meta, 1).unwrap(),
            IntegrityResult::Valid
        );
    }

    #[test]
    fn verify_composite_tampered_metadata_is_invalid() {
        let mut dag = MerkleDAG::new(ObjectIdentity::compute(b"root"));
        let c1 = make_id(b"child one");
        let id = MerkleDAG::composite_identity(b"meta", &[c1], 1);
        dag.insert_composite(id, NodeType::Internal, &[c1], b"meta".to_vec(), 1, 0);
        let verifier = IntegrityVerifier::new(&dag);
        assert_eq!(
            verifier.verify_composite(&id, b"tampered", 1).unwrap(),
            IntegrityResult::Invalid
        );
    }

    #[test]
    fn incremental_recompute_only_affects_path() {
        let mut dag = MerkleDAG::new(ObjectIdentity::compute(b"root"));
        let leaf = make_id(b"leaf content");
        dag.insert_leaf(leaf, 12);
        let root = make_id(b"root content");
        dag.insert_composite(
            root,
            NodeType::Internal,
            &[leaf],
            b"root meta".to_vec(),
            1,
            0,
        );
        dag.root = root;

        // Change the leaf's content -> new identity.
        let new_leaf = make_id(b"modified leaf content");
        let new_root = dag.recompute_root(&leaf, &new_leaf).unwrap();
        // The new root differs because the child changed.
        assert_ne!(new_root, root);
        assert_eq!(
            new_root,
            MerkleDAG::composite_identity(b"root meta", &[new_leaf], 1)
        );
    }

    #[test]
    fn signed_integrity_valid_when_non_zero() {
        let signed = SignedIntegrity::new(make_id(b"root"), b"key-1".to_vec(), [0x42u8; 64], 1);
        assert_eq!(signed.verify(), IntegrityResult::Valid);
    }

    #[test]
    fn signed_integrity_invalid_when_zero_signature() {
        let signed = SignedIntegrity::new(make_id(b"root"), b"key-1".to_vec(), [0u8; 64], 1);
        assert_eq!(signed.verify(), IntegrityResult::Invalid);
    }

    #[test]
    fn depth_limit_is_enforced() {
        let mut dag = MerkleDAG::new(ObjectIdentity::compute(b"root"));
        dag.max_depth = 1;
        let leaf = make_id(b"leaf");
        dag.insert_leaf(leaf, 4);
        // A chain deeper than max_depth should fail verification.
        let c1 = make_id(b"c1");
        dag.insert_composite(c1, NodeType::Internal, &[leaf], b"m".to_vec(), 0, 1);
        let c2 = make_id(b"c2");
        dag.insert_composite(c2, NodeType::Internal, &[c1], b"m".to_vec(), 0, 1);
        dag.root = c2;
        assert!(dag.verify_depth().is_err());
    }
}
