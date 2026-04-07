use std::collections::{HashMap, HashSet, VecDeque};

use serde::Serialize;
use turso::Connection;

use crate::db::query_all;
use crate::error::Result;

/// A room node in the palace graph.
#[derive(Debug, Clone, Serialize)]
pub struct RoomNode {
    pub room: String,
    pub wings: Vec<String>,
    pub count: usize,
}

/// A tunnel edge: a room that spans multiple wings.
#[derive(Debug, Clone, Serialize)]
pub struct TunnelEdge {
    pub room: String,
    pub wing_a: String,
    pub wing_b: String,
    pub count: usize,
}

/// A traversal result entry.
#[derive(Debug, Clone, Serialize)]
pub struct TraversalResult {
    pub room: String,
    pub wings: Vec<String>,
    pub count: usize,
    pub hop: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connected_via: Option<Vec<String>>,
}

/// Palace graph statistics.
#[derive(Debug, Clone, Serialize)]
pub struct GraphStats {
    pub total_rooms: usize,
    pub tunnel_rooms: usize,
    pub total_edges: usize,
    pub rooms_per_wing: HashMap<String, usize>,
    pub top_tunnels: Vec<RoomNode>,
}

/// Build the palace graph from drawer metadata.
/// Returns (nodes, edges) where nodes are rooms and edges are tunnels.
pub async fn build_graph(
    conn: &Connection,
) -> Result<(HashMap<String, RoomNode>, Vec<TunnelEdge>)> {
    let rows = query_all(
        conn,
        "SELECT room, wing, COUNT(*) as cnt FROM drawers WHERE room != 'general' AND room != '' GROUP BY room, wing",
        (),
    )
    .await?;

    // Aggregate room data across wings
    let mut room_data: HashMap<String, (HashSet<String>, usize)> = HashMap::new();
    for row in &rows {
        let room: String = row.get(0)?;
        let wing: String = row.get(1)?;
        let count: i64 = row.get(2)?;
        let entry = room_data.entry(room).or_insert_with(|| (HashSet::new(), 0));
        entry.0.insert(wing);
        entry.1 += usize::try_from(count).unwrap_or(0);
    }

    // Build nodes
    let mut nodes = HashMap::new();
    for (room, (wings, count)) in &room_data {
        let mut wing_list: Vec<String> = wings.iter().cloned().collect();
        wing_list.sort();
        nodes.insert(
            room.clone(),
            RoomNode {
                room: room.clone(),
                wings: wing_list,
                count: *count,
            },
        );
    }

    // Build edges from rooms spanning multiple wings
    let mut edges = Vec::new();
    for (room, (wings, count)) in &room_data {
        let mut wing_list: Vec<&String> = wings.iter().collect();
        wing_list.sort();
        if wing_list.len() >= 2 {
            for (i, wa) in wing_list.iter().enumerate() {
                for wb in &wing_list[i + 1..] {
                    edges.push(TunnelEdge {
                        room: room.clone(),
                        wing_a: (*wa).clone(),
                        wing_b: (*wb).clone(),
                        count: *count,
                    });
                }
            }
        }
    }

    Ok((nodes, edges))
}

/// BFS traversal from a starting room. Find connected rooms through shared wings.
pub async fn traverse(
    conn: &Connection,
    start_room: &str,
    max_hops: usize,
) -> Result<Vec<TraversalResult>> {
    let (nodes, _) = build_graph(conn).await?;

    let start = match nodes.get(start_room) {
        Some(node) => node.clone(),
        None => return Ok(Vec::new()),
    };

    let mut visited = HashSet::new();
    visited.insert(start_room.to_string());

    let mut results = vec![TraversalResult {
        room: start.room.clone(),
        wings: start.wings.clone(),
        count: start.count,
        hop: 0,
        connected_via: None,
    }];

    let mut frontier: VecDeque<(String, usize)> = VecDeque::new();
    frontier.push_back((start_room.to_string(), 0));

    while let Some((current_room, depth)) = frontier.pop_front() {
        if depth >= max_hops {
            continue;
        }

        let current_wings: HashSet<String> = nodes
            .get(&current_room)
            .map(|n| n.wings.iter().cloned().collect())
            .unwrap_or_default();

        for (room, node) in &nodes {
            if visited.contains(room) {
                continue;
            }
            let node_wings: HashSet<String> = node.wings.iter().cloned().collect();
            let shared: Vec<String> = current_wings.intersection(&node_wings).cloned().collect();
            if !shared.is_empty() {
                visited.insert(room.clone());
                let mut sorted_shared = shared;
                sorted_shared.sort();
                results.push(TraversalResult {
                    room: room.clone(),
                    wings: node.wings.clone(),
                    count: node.count,
                    hop: depth + 1,
                    connected_via: Some(sorted_shared),
                });
                if depth + 1 < max_hops {
                    frontier.push_back((room.clone(), depth + 1));
                }
            }
        }
    }

    results.sort_by(|a, b| a.hop.cmp(&b.hop).then_with(|| b.count.cmp(&a.count)));
    results.truncate(50);
    Ok(results)
}

/// Find rooms that connect two wings (tunnels).
pub async fn find_tunnels(
    conn: &Connection,
    wing_a: Option<&str>,
    wing_b: Option<&str>,
) -> Result<Vec<RoomNode>> {
    let (nodes, _) = build_graph(conn).await?;

    let mut tunnels: Vec<RoomNode> = nodes
        .into_values()
        .filter(|node| {
            if node.wings.len() < 2 {
                return false;
            }
            if let Some(wa) = wing_a
                && !node.wings.contains(&wa.to_string())
            {
                return false;
            }
            if let Some(wb) = wing_b
                && !node.wings.contains(&wb.to_string())
            {
                return false;
            }
            true
        })
        .collect();

    tunnels.sort_by(|a, b| b.count.cmp(&a.count));
    tunnels.truncate(50);
    Ok(tunnels)
}

/// Summary statistics about the palace graph.
pub async fn graph_stats(conn: &Connection) -> Result<GraphStats> {
    let (nodes, edges) = build_graph(conn).await?;

    let tunnel_rooms = nodes.values().filter(|n| n.wings.len() >= 2).count();

    let mut wing_counts: HashMap<String, usize> = HashMap::new();
    for node in nodes.values() {
        for w in &node.wings {
            *wing_counts.entry(w.clone()).or_insert(0) += 1;
        }
    }

    let mut top_tunnels: Vec<RoomNode> = nodes
        .values()
        .filter(|n| n.wings.len() >= 2)
        .cloned()
        .collect();
    top_tunnels.sort_by(|a, b| b.wings.len().cmp(&a.wings.len()));
    top_tunnels.truncate(10);

    Ok(GraphStats {
        total_rooms: nodes.len(),
        tunnel_rooms,
        total_edges: edges.len(),
        rooms_per_wing: wing_counts,
        top_tunnels,
    })
}
