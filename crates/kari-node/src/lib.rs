use std::collections::HashSet;

struct Node {
    id: u32,
    connected_nodes: HashSet<u32>, // ใช้ HashSet เพื่อป้องกันการเชื่อมต่อซ้ำซ้อน
    latest_block: u32,
    has_issue: bool,
}

impl Node {
    fn new(id: u32, latest_block: u32) -> Self {
        Node {
            id,
            connected_nodes: HashSet::new(),
            latest_block,
            has_issue: false,
        }
    }

    fn connect(&mut self, other: &mut Node) {
        // เพิ่มการตรวจสอบว่า node ไม่ได้เชื่อมต่อกับตัวเอง
        if self.id != other.id {
            self.connected_nodes.insert(other.id);
            other.connected_nodes.insert(self.id);
        }
    }

    fn handle_issue(&mut self) {
        if self.has_issue {
            // ตรวจสอบว่ามี node อื่นที่เชื่อมต่ออยู่หรือไม่
            if !self.connected_nodes.is_empty() {
                self.run_latest_block();
            } else {
                println!("Node {} is isolated and cannot resolve the issue.", self.id);
            }
        }
    }

    fn run_latest_block(&mut self) {
        println!("Node {} is running the latest block: {}", self.id, self.latest_block);
        self.has_issue = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_node() {
        let node = Node::new(1, 100);
        assert_eq!(node.id, 1);
        assert_eq!(node.latest_block, 100);
        assert!(node.connected_nodes.is_empty());
        assert!(!node.has_issue);
    }

    #[test]
    fn test_connect_nodes() {
        let mut node1 = Node::new(1, 100);
        let mut node2 = Node::new(2, 100);

        node1.connect(&mut node2);

        assert!(node1.connected_nodes.contains(&2));
        assert!(node2.connected_nodes.contains(&1));
    }

    #[test]
    fn test_handle_issue() {
        let mut node1 = Node::new(1, 100);
        let mut node2 = Node::new(2, 100);
        node1.connect(&mut node2);
        node1.has_issue = true;

        node1.handle_issue();

        assert!(!node1.has_issue);
    }

    #[test]
    fn test_handle_issue_isolated() {
        let mut node = Node::new(1, 100);
        node.has_issue = true;

        node.handle_issue();

        assert!(node.has_issue); // Node remains with the issue
    }

    #[test]
    fn test_run_latest_block() {
        let mut node = Node::new(1, 100);
        node.has_issue = true;

        node.run_latest_block();

        assert!(!node.has_issue);
    }
}
