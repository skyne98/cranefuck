use thiserror::Error;

use crate::parser::{Ir, IrLoopType};
use crate::peephole::PeepholeIr;
use anyhow::Result;
use std::collections::HashSet;
use std::fmt::{self, Debug, Formatter};

// Structures
// ==========
pub type TreeId = usize;
#[derive(Debug, Clone)]
pub enum TreeNodeType {
    Ir(PeepholeIr),
    Loop(TreeId),
    Sequence(Vec<TreeId>),
}
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: TreeId,
    pub node_type: TreeNodeType,
}
#[derive(Clone)]
pub struct Tree {
    pub nodes: Vec<TreeNode>,
    pub root: TreeId,
}
impl Tree {
    pub fn new() -> Self {
        Tree {
            nodes: Vec::new(),
            root: 0,
        }
    }
}
impl Debug for Tree {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Tree {{")?;
        self.fmt_node(self.root, f, 1, &mut HashSet::new())?;
        writeln!(f, "}}")
    }
}
impl Tree {
    fn fmt_node(
        &self,
        id: TreeId,
        f: &mut Formatter<'_>,
        depth: usize,
        visited: &mut HashSet<TreeId>,
    ) -> fmt::Result {
        if !visited.insert(id) {
            return writeln!(f, "{}Node {}: <cycle>", "  ".repeat(depth), id);
        }

        let indent = "  ".repeat(depth);
        let node = &self.nodes[id];

        match &node.node_type {
            TreeNodeType::Ir(ir) => {
                writeln!(f, "{}Node {}: {:?}", indent, id, ir)
            }
            TreeNodeType::Loop(inner_id) => {
                writeln!(f, "{}Node {}: Loop [", indent, id)?;
                self.fmt_node(*inner_id, f, depth + 1, visited)?;
                writeln!(f, "{}]", indent)
            }
            TreeNodeType::Sequence(seq) => {
                writeln!(f, "{}Node {}: Sequence [", indent, id)?;
                for &child_id in seq {
                    self.fmt_node(child_id, f, depth + 1, visited)?;
                }
                writeln!(f, "{}]", indent)
            }
        }
    }
}

#[derive(Error, Debug)]
pub enum TreeError {
    #[error("unexpected end of input while parsing")]
    UnexpectedEof,
}

// Tree building
// =============
pub fn build_tree(ir_ops: impl AsRef<[PeepholeIr]>) -> Result<Tree, TreeError> {
    let mut tree = Tree::new();
    let mut ops = ir_ops.as_ref().iter().peekable();
    tree.root = build_tree_inner(&mut tree, &mut ops)?;
    Ok(tree)
}

fn build_tree_inner(
    tree: &mut Tree,
    ops: &mut std::iter::Peekable<std::slice::Iter<PeepholeIr>>,
) -> Result<TreeId, TreeError> {
    let mut sequence = Vec::new();

    while let Some(op) = ops.peek() {
        match op {
            PeepholeIr::Ir(Ir::Loop(IrLoopType::End, _)) => {
                ops.next();
                break;
            }
            PeepholeIr::Ir(Ir::Loop(_, _)) => {
                ops.next();
                let inner_id = build_tree_inner(tree, ops)?;
                let loop_id = tree.nodes.len();
                tree.nodes.push(TreeNode {
                    id: loop_id,
                    node_type: TreeNodeType::Loop(inner_id),
                });
                sequence.push(loop_id);
            }
            _ => {
                let ir_id = tree.nodes.len();
                tree.nodes.push(TreeNode {
                    id: ir_id,
                    node_type: TreeNodeType::Ir(
                        ops.next().ok_or(TreeError::UnexpectedEof)?.clone(),
                    ),
                });
                sequence.push(ir_id);
            }
        }
    }

    let sequence_id = tree.nodes.len();
    tree.nodes.push(TreeNode {
        id: sequence_id,
        node_type: TreeNodeType::Sequence(sequence),
    });
    Ok(sequence_id)
}
