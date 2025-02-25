use petgraph::algo::{astar, is_cyclic_directed, kosaraju_scc};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction::{Incoming, Outgoing};
use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub struct NodeData {
//     pub name: String,
//     pub value: i32,
//     pub metadata: String,
// }

#[derive(Debug, Clone)]
pub struct DirectedGraph<T>
where
    T: Eq + Hash + Clone,
{
    graph: DiGraph<T, ()>,
    node_indices: HashMap<String, NodeIndex>,
}

impl<T> DirectedGraph<T>
where
    T: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, name: String, data: T) {
        let idx = self.graph.add_node(data);
        self.node_indices.insert(name, idx);
    }

    pub fn add_connection(&mut self, from: &str, to: &str) {
        if let (Some(&f_idx), Some(&t_idx)) =
            (self.node_indices.get(from), self.node_indices.get(to))
        {
            self.graph.add_edge(f_idx, t_idx, ());
        }
    }

    /// 删除节点，若被 swap 删除，则修正被置换节点索引，最后重连 父 -> 子
    pub fn remove_node(&mut self, name: &str) {
        if let Some(&idx) = self.node_indices.get(name) {
            let parent_names: Vec<String> = self
                .graph
                .neighbors_directed(idx, Incoming)
                .filter_map(|p_idx| self.index_to_name(p_idx))
                .collect();

            let child_names: Vec<String> = self
                .graph
                .neighbors_directed(idx, Outgoing)
                .filter_map(|c_idx| self.index_to_name(c_idx))
                .collect();

            // swap-removal
            let last_index = NodeIndex::new(self.graph.node_count() - 1);
            self.graph.remove_node(idx);
            self.node_indices.remove(name);

            if idx != last_index && self.graph.node_count() > 0 {
                // 修正被 swap 到 idx 的节点
                for (n, stored_idx) in self.node_indices.iter_mut() {
                    if *stored_idx == last_index {
                        *stored_idx = idx;
                        break;
                    }
                }
            }

            // 父 -> 子 直连
            for p_name in parent_names {
                for c_name in &child_names {
                    self.add_connection(&p_name, c_name);
                }
            }
        }
    }

    pub fn get_direct_children(&self, name: &str) -> Vec<&T> {
        if let Some(&idx) = self.node_indices.get(name) {
            self.graph
                .neighbors(idx)
                .filter_map(|child_idx| self.graph.node_weight(child_idx))
                .collect()
        } else {
            vec![]
        }
    }

    pub fn get_direct_parents(&self, name: &str) -> Vec<&T> {
        if let Some(&idx) = self.node_indices.get(name) {
            self.graph
                .neighbors_directed(idx, Incoming)
                .filter_map(|p_idx| self.graph.node_weight(p_idx))
                .collect()
        } else {
            vec![]
        }
    }

    pub fn is_dag(&self) -> bool {
        !is_cyclic_directed(&self.graph)
    }

    pub fn find_strongly_connected_components(&self) -> Vec<HashSet<&T>> {
        let sccs = kosaraju_scc(&self.graph);
        sccs.into_iter()
            .map(|comp| {
                comp.into_iter()
                    .filter_map(|idx| self.graph.node_weight(idx))
                    .collect()
            })
            .collect()
    }

    /// A* 找最短路径
    pub fn find_shortest_path(&self, start: &str, target: &str) -> Option<(Vec<String>, usize)> {
        if let (Some(&start_idx), Some(&target_idx)) =
            (self.node_indices.get(start), self.node_indices.get(target))
        {
            if let Some((cost, path_idxes)) = astar(
                &self.graph,
                start_idx,
                |finish| finish == target_idx,
                |_| 1,
                |_| 0,
            ) {
                let path_names: Vec<String> = path_idxes
                    .into_iter()
                    .filter_map(|i| self.index_to_name(i))
                    .collect();
                return Some((path_names, cost));
            }
        }
        None
    }

    /// 导出简单的工作流结构: Vec<(node_name, [children])>
    pub fn to_workflow(&self) -> Vec<(String, Vec<String>)> {
        let mut result = Vec::new();
        for (name, &idx) in &self.node_indices {
            let children: Vec<String> = self
                .graph
                .neighbors(idx)
                .filter_map(|c_idx| self.index_to_name(c_idx))
                .collect();
            result.push((name.clone(), children));
        }
        result
    }

    /// 从 workflow 导入
    pub fn from_workflow<F>(workflow: Vec<(String, Vec<String>)>, mut to_t: F) -> Self
    where
        F: FnMut(&str) -> T,
    {
        let mut g = DirectedGraph::new();
        for (name, _) in &workflow {
            let data_t = to_t(name);
            g.add_node(name.clone(), data_t);
        }
        for (name, children) in workflow {
            for c in children {
                g.add_connection(&name, &c);
            }
        }
        g
    }

    /// 帮助方法: 通过 NodeIndex 找到节点名称
    fn index_to_name(&self, idx: NodeIndex) -> Option<String> {
        self.node_indices.iter().find_map(|(n, &stored_idx)| {
            if stored_idx == idx {
                Some(n.clone())
            } else {
                None
            }
        })
    }
}

// ========================
// 附加功能示例
// ========================
impl<T> DirectedGraph<T>
where
    T: Eq + Hash + Clone,
{
    /// 1. 获取所有 "根节点" (无父节点) 名称
    pub fn get_root_nodes(&self) -> Vec<String> {
        let mut roots = Vec::new();
        for (name, &idx) in &self.node_indices {
            // 如果没有任何 parent, 就是 root
            let parent_count = self.graph.neighbors_directed(idx, Incoming).count();
            if parent_count == 0 {
                roots.push(name.clone());
            }
        }
        roots
    }

    /// 2. 获取所有 "叶节点" (无子节点) 名称
    pub fn get_leaf_nodes(&self) -> Vec<String> {
        let mut leaves = Vec::new();
        for (name, &idx) in &self.node_indices {
            // 如果没有任何 child, 就是 leaf
            let child_count = self.graph.neighbors(idx).count();
            if child_count == 0 {
                leaves.push(name.clone());
            }
        }
        leaves
    }

    /// 3. Kahn 算法进行拓扑排序, 返回节点名称的有序列表
    /// 如果图有环, 会 Err("Graph has a cycle")
    pub fn topological_sort(&self) -> Result<Vec<String>, &'static str> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        // 初始化 in-degree
        for (name, _) in &self.node_indices {
            in_degree.insert(name.clone(), 0);
        }
        // 统计每个节点的入度
        for (name, &idx) in &self.node_indices {
            // 这个节点的所有子节点 in-degree +=1
            for child_idx in self.graph.neighbors(idx) {
                if let Some(child_name) = self.index_to_name(child_idx) {
                    let counter = in_degree.entry(child_name).or_insert(0);
                    *counter += 1;
                }
            }
        }
        // 找到所有入度=0 的节点, 推入队列
        let mut queue = VecDeque::new();
        for (name, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(name.clone());
            }
        }

        let mut result = Vec::new();
        while let Some(node_name) = queue.pop_front() {
            result.push(node_name.clone());
            // 所有直接子节点 in-degree 减1
            if let Some(&idx) = self.node_indices.get(&node_name) {
                for child_idx in self.graph.neighbors(idx) {
                    if let Some(child_name) = self.index_to_name(child_idx) {
                        let counter = in_degree.get_mut(&child_name).unwrap();
                        *counter -= 1;
                        if *counter == 0 {
                            queue.push_back(child_name);
                        }
                    }
                }
            }
        }
        // 如果排序后的数量 < 节点总数, 说明有环
        if result.len() < self.node_indices.len() {
            return Err("Graph has a cycle");
        }
        Ok(result)
    }

    /// 4. 移除特定的一条边 (from->to)
    pub fn remove_connection(&mut self, from: &str, to: &str) {
        if let (Some(&f_idx), Some(&t_idx)) =
            (self.node_indices.get(from), self.node_indices.get(to))
        {
            // petgraph 没有直接 `remove_edge` by node index(0.6.x 没有),
            // 需要先找 edge_id，再 remove
            if let Some(edge_id) = self.graph.find_edge(f_idx, t_idx) {
                self.graph.remove_edge(edge_id);
            }
        }
    }

    /// 5. 更新一条连接 (例如修改 from->to 改成 from->newTo),
    /// 或者把 from->to 替换成新的 from2->to2, 看业务需求
    pub fn update_connection(
        &mut self,
        old_from: &str,
        old_to: &str,
        new_from: &str,
        new_to: &str,
    ) {
        // 移除旧连接
        self.remove_connection(old_from, old_to);
        // 添加新连接
        self.add_connection(new_from, new_to);
    }

    /// 6. 获取所有后代(递归子节点)名称
    pub fn get_all_children(&self, name: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        if let Some(&start_idx) = self.node_indices.get(name) {
            self.dfs_children(start_idx, &mut visited);
        }
        visited
    }

    fn dfs_children(&self, current_idx: NodeIndex, visited: &mut HashSet<String>) {
        for child_idx in self.graph.neighbors(current_idx) {
            if let Some(child_name) = self.index_to_name(child_idx) {
                if visited.insert(child_name.clone()) {
                    // 如果成功插入 (之前没见过), 继续 DFS
                    self.dfs_children(child_idx, visited);
                }
            }
        }
    }

    /// 7. 获取所有祖先(递归父节点)名称
    pub fn get_all_parents(&self, name: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        if let Some(&start_idx) = self.node_indices.get(name) {
            self.dfs_parents(start_idx, &mut visited);
        }
        visited
    }

    fn dfs_parents(&self, current_idx: NodeIndex, visited: &mut HashSet<String>) {
        for parent_idx in self.graph.neighbors_directed(current_idx, Incoming) {
            if let Some(parent_name) = self.index_to_name(parent_idx) {
                if visited.insert(parent_name.clone()) {
                    // 继续往上爬
                    self.dfs_parents(parent_idx, visited);
                }
            }
        }
    }

    /// 8. 重命名节点 (修改 node_indices key),
    ///    同时节点自身 T 如果有 name 字段也需要自行更新
    pub fn rename_node(&mut self, old_name: &str, new_name: &str) {
        if let Some(&idx) = self.node_indices.get(old_name) {
            // 1) 先从 HashMap 移除旧 key
            self.node_indices.remove(old_name);
            // 2) 插入新 key -> 同一个 idx
            self.node_indices.insert(new_name.to_string(), idx);

            // 3) 如果你的 T 内部有“name”字段, 需要修改 graph.node_weight_mut(idx)
            if let Some(weight) = self.graph.node_weight_mut(idx) {
                // 这里视具体 T 的实现而定:
                // 比如 T = NodeData { name, ... } => weight.name = new_name.to_string();
                // 或者 T = String => weight = new_name.to_string();
            }
        }
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::collections::HashSet;

//     /// 测试用自定义结构
//     #[derive(Debug, Clone, PartialEq, Eq, Hash)]
//     pub struct NodeDataTest {
//         pub name: String,
//         pub value: i32,
//     }

//     /// 1. 测试 from_workflow + T=String
//     #[test]
//     fn test_from_workflow_with_string() {
//         let workflow = vec![
//             ("A".to_string(), vec!["B".to_string(), "C".to_string()]),
//             ("B".to_string(), vec!["D".to_string()]),
//             ("C".to_string(), vec!["D".to_string()]),
//             ("D".to_string(), vec![]),
//         ];

//         let graph: DirectedGraph<String> =
//             DirectedGraph::from_workflow(workflow, |s| s.to_string());

//         let children_a = graph.get_direct_children("A");
//         // children_a is Vec<&String>
//         let child_set: HashSet<String> = children_a.iter().map(|s| (*s).clone()).collect();
//         assert_eq!(
//             child_set,
//             ["B".to_string(), "C".to_string()].iter().cloned().collect()
//         );
//     }

//     /// 2. 测试 from_workflow + T=自定义结构
//     #[test]
//     fn test_from_workflow_with_nodedata() {
//         let workflow = vec![
//             ("X".to_string(), vec!["Y".to_string()]),
//             ("Y".to_string(), vec![]),
//         ];

//         // 用 NodeDataTest 来构造 DirectedGraph
//         let graph: DirectedGraph<NodeDataTest> =
//             DirectedGraph::from_workflow(workflow, |name| NodeDataTest {
//                 name: name.to_owned(),
//                 value: 42,
//             });

//         assert!(graph.is_dag());

//         let x_children = graph.get_direct_children("X");
//         assert_eq!(x_children.len(), 1);
//         assert_eq!(x_children[0].name, "Y");
//         assert_eq!(x_children[0].value, 42);
//     }

//     /// 3. 测试删除节点并自动让父子节点相连
//     #[test]
//     fn test_add_and_remove_node() {
//         let mut graph: DirectedGraph<String> = DirectedGraph::new();
//         graph.add_node("A".into(), "A".into());
//         graph.add_node("B".into(), "B".into());
//         graph.add_node("C".into(), "C".into());
//         // A -> B -> C
//         graph.add_connection("A", "B");
//         graph.add_connection("B", "C");

//         // 删除 B => A 直接连到 C
//         graph.remove_node("B");

//         // A 的直接子节点应是 C
//         let a_children = graph.get_direct_children("A");
//         let set: HashSet<String> = a_children.into_iter().cloned().collect();
//         assert_eq!(set, ["C".to_string()].iter().cloned().collect());
//     }

//     /// 4. 测试最短路径
//     #[test]
//     fn test_find_shortest_path() {
//         let mut graph: DirectedGraph<String> = DirectedGraph::new();
//         graph.add_node("A".into(), "A".into());
//         graph.add_node("B".into(), "B".into());
//         graph.add_node("C".into(), "C".into());
//         graph.add_node("D".into(), "D".into());

//         graph.add_connection("A", "B");
//         graph.add_connection("B", "C");
//         graph.add_connection("C", "D");

//         if let Some((path, cost)) = graph.find_shortest_path("A", "D") {
//             assert_eq!(path, vec!["A", "B", "C", "D"]);
//             assert_eq!(cost, 3);
//         } else {
//             panic!("没有找到最短路径");
//         }
//     }

//     #[test]
//     fn test_topological_sort() {
//         let mut graph: DirectedGraph<String> = DirectedGraph::new();
//         graph.add_node("A".to_string(), "A".to_string());
//         graph.add_node("B".to_string(), "B".to_string());
//         graph.add_node("C".to_string(), "C".to_string());
//         graph.add_connection("A", "B");
//         graph.add_connection("A", "C");
//         graph.add_connection("B", "C");

//         let sorted = graph.topological_sort().unwrap();
//         // e.g. possible order: A->B->C or A->C->B
//         assert_eq!(sorted[0], "A");
//     }
// }


#[test]
fn test_complex_nodedata_workflow() {
    use super::*;
    use alphaflow_nodes::node::Node;

    // 1. 构造一个 DirectedGraph<NodeData>
    let mut graph: DirectedGraph<Node> = DirectedGraph::new();

    // 2. 添加一些节点 (name, value, metadata 可根据需求自定义)
    graph.add_node(
        "TaskA".into(),
        Node {
            name: "TaskA".into(),
            node_type_name: "http".into(),
            disabled: false,
            input_mapping: None,
            parameters: json!({}),
            display_name: None,
            description: None,
            custom_config: None,
        },
    );
    graph.add_node(
        "TaskB".into(),
        Node {
            name: "TaskB".into(),
            node_type_name: "http".into(),
            disabled: false,
            input_mapping: None,
            parameters: json!({}),
            display_name: None,
            description: None,
            custom_config: None,
        },
    );
    graph.add_node(
        "TaskC".into(),
        Node {
            name: "TaskC".into(),
            node_type_name: "http".into(),
            disabled: false,  
            input_mapping: None,
            parameters: json!({}),
            display_name: None,
            description: None,
            custom_config: None,
        },
    );
    graph.add_node(
        "TaskD".into(),
        Node {
            name: "TaskD".into(),
            node_type_name: "http".into(),
            disabled: false,
            input_mapping: None,
            parameters: json!({}),
            display_name: None,
            description: None,
            custom_config: None,
        },
    );

    // 3. 添加一些连接 (构造一个简单的有向无环图)
    //    A -> B, A -> C, B -> D, C -> D
    graph.add_connection("TaskA", "TaskB");
    graph.add_connection("TaskA", "TaskC");
    graph.add_connection("TaskB", "TaskD");
    graph.add_connection("TaskC", "TaskD");

    // 4. 验证是否是 DAG
    assert!(graph.is_dag(), "初始图应当是无环的");

    // 5. 获取拓扑排序结果 (Kahn 算法)
    let topo_order = graph.topological_sort().expect("应当能进行拓扑排序");
    // 可能的顺序 ["TaskA", "TaskB", "TaskC", "TaskD"] 或 ["TaskA", "TaskC", "TaskB", "TaskD"]
    assert_eq!(topo_order.first().unwrap(), "TaskA"); // A 一定在最前面

    // 6. 查看 TaskA 的直接子节点
    let direct_children_of_a = graph.get_direct_children("TaskA");
    assert_eq!(direct_children_of_a.len(), 2);
    // 打印一下，看看节点数据
    println!("TaskA 的直接子节点: {:?}", direct_children_of_a);

    // 7. 获取 TaskD 的所有祖先
    let all_parents_of_d = graph.get_all_parents("TaskD");
    // 应该包含 A、B、C
    println!("TaskD 的所有祖先: {:?}", all_parents_of_d);
    assert!(all_parents_of_d.contains("TaskA"));
    assert!(all_parents_of_d.contains("TaskB"));
    assert!(all_parents_of_d.contains("TaskC"));

    // 8. 移除节点 TaskC，并观察 A -> B, B->D 是否依旧存在，且 A -> D 会被自动直连。
    //    原图: A -> C -> D, 移除 C 后，A->D 将被“父-子直连”。
    graph.remove_node("TaskC");

    // 移除 C 后，TaskA 的直接子节点里就多了 "TaskD"
    let direct_children_of_a_after = graph.get_direct_children("TaskA");
    println!(
        "移除TaskC后 TaskA 的直接子节点: {:?}",
        direct_children_of_a_after
    );
    assert_eq!(direct_children_of_a_after.len(), 2);
    // 现在应该有 ["TaskB", "TaskD"]

    // 9. 重命名节点: TaskB -> TaskB2 (并更新内部 NodeData)
    graph.rename_node("TaskB", "TaskB2");
    if let Some(b2_parents) = graph.get_direct_parents("TaskB2").first() {
        println!("TaskB2 的父节点: {:?}", b2_parents);
    }

    // 同时也更新内部的节点数据(假设真的需要的话):
    if let Some(&idx_b2) = graph.node_indices.get("TaskB2") {
        if let Some(weight) = graph.graph.node_weight_mut(idx_b2) {
            weight.name = "TaskB2".to_string(); // 更新 name 字段
        }
    }

    // 再次查看拓扑排序
    let topo_order2 = graph.topological_sort().expect("此时依旧应当是无环图");
    println!("拓扑排序(重命名后): {:?}", topo_order2);
    // 依旧要包含 ["TaskA", "TaskB2", "TaskD"]

    // 10. 查看强连通分量(当前图无环，每个节点都是独立的强连通分量)
    let sccs = graph.find_strongly_connected_components();
    println!("SCC 分析: {:?}", sccs);
    // 对于无环图，SCC 是若干个单节点组成的集合。
    for comp in &sccs {
        assert_eq!(comp.len(), 1, "无环时, 每个强连通分量只会包含1个节点");
    }

    // 11. 最终断言一下，图中应该剩余节点是3个: ["TaskA", "TaskB2", "TaskD"]
    assert_eq!(graph.node_indices.len(), 3);
    assert!(graph.node_indices.contains_key("TaskA"));
    assert!(graph.node_indices.contains_key("TaskB2"));
    assert!(graph.node_indices.contains_key("TaskD"));

    // 如果愿意，也可以再演示添加环、判断出现 cycle 之类，这里暂不展开。
    println!("测试完成: 复杂结构节点的工作流图操作验证通过！");
}



