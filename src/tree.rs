use anyhow::Result;
use std::fmt;
use thiserror::Error;

use crate::parser::{Ir, IrLoopType};
use crate::peephole::PeepholeIr;

/// A unique identifier for nodes in the abstract syntax tree
pub type NodeId = usize;

/// Represents the different types of nodes in the abstract syntax tree
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// A primitive instruction
    Instruction(PeepholeIr),
    /// A loop containing another node
    Loop(NodeId),
    /// A sequence of nodes
    Sequence(Vec<NodeId>),
}

/// A node in the abstract syntax tree
#[derive(Debug, Clone)]
pub struct Node {
    /// The unique identifier for this node
    pub id: NodeId,
    /// The kind of node
    pub kind: NodeKind,
}

/// The complete abstract syntax tree
#[derive(Clone)]
pub struct Tree {
    nodes: Vec<Node>,
    root: NodeId,
}

#[derive(Error, Debug)]
pub enum TreeError {
    #[error("unexpected end of input while parsing")]
    UnexpectedEof,
}

impl Tree {
    /// Creates a new empty tree
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            root: 0,
        }
    }

    pub fn add_node(&mut self, kind: NodeKind) -> NodeId {
        let id = self.nodes.len();
        self.nodes.push(Node { id, kind });
        id
    }
    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }
    pub fn root(&self) -> NodeId {
        self.root
    }
    pub fn children(&self, id: NodeId) -> Option<&[NodeId]> {
        match self.get(id).map(|node| &node.kind) {
            Some(NodeKind::Sequence(seq)) => Some(seq),
            Some(NodeKind::Loop(inner_id)) => {
                let inner_node = self.get(*inner_id).unwrap();
                match &inner_node.kind {
                    NodeKind::Sequence(seq) => Some(seq),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl fmt::Debug for Tree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct DebugTree<'a>(&'a Tree, NodeId, usize);

        impl<'a> fmt::Debug for DebugTree<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let tree = self.0;
                let id = self.1;
                let depth = self.2;
                let indent = "  ".repeat(depth);

                match tree.get(id).map(|node| &node.kind) {
                    Some(NodeKind::Instruction(ir)) => {
                        write!(f, "{indent}Instruction: {:?}", ir)
                    }
                    Some(NodeKind::Loop(inner_id)) => {
                        writeln!(f, "{indent}Loop [")?;
                        write!(f, "{:?}", DebugTree(tree, *inner_id, depth + 1))?;
                        writeln!(f, "")?; // Add newline before closing bracket
                        write!(f, "{indent}]")
                    }
                    Some(NodeKind::Sequence(seq)) => {
                        writeln!(f, "{indent}Sequence [")?;
                        for &child_id in seq {
                            writeln!(f, "{:?}", DebugTree(tree, child_id, depth + 1))?;
                        }
                        write!(f, "{indent}]")
                    }
                    None => write!(f, "{indent}<invalid node>"),
                }
            }
        }

        writeln!(f, "Tree {{")?;
        writeln!(f, "{:?}", DebugTree(self, self.root, 1))?;
        writeln!(f, "}}")
    }
}

/// Builds an abstract syntax tree from a sequence of IR operations
pub fn build_tree(ir_ops: impl AsRef<[PeepholeIr]>) -> Result<Tree, TreeError> {
    let mut tree = Tree::new();
    let mut ops = ir_ops.as_ref().iter().peekable();
    tree.root = parse_sequence(&mut tree, &mut ops)?;
    Ok(tree)
}

fn parse_sequence(
    tree: &mut Tree,
    ops: &mut std::iter::Peekable<std::slice::Iter<PeepholeIr>>,
) -> Result<NodeId, TreeError> {
    let mut sequence = Vec::new();

    while let Some(op) = ops.peek() {
        match op {
            PeepholeIr::Ir(Ir::Loop(IrLoopType::End, _)) => {
                ops.next(); // Consume the end loop marker
                break;
            }
            PeepholeIr::Ir(Ir::Loop(_, _)) => {
                ops.next(); // Consume the start loop marker
                let inner_id = parse_sequence(tree, ops)?;
                let loop_id = tree.add_node(NodeKind::Loop(inner_id));
                sequence.push(loop_id);
            }
            _ => {
                let op_clone = ops.next().ok_or(TreeError::UnexpectedEof)?.clone();
                let node_id = tree.add_node(NodeKind::Instruction(op_clone));
                sequence.push(node_id);
            }
        }
    }

    Ok(tree.add_node(NodeKind::Sequence(sequence)))
}
