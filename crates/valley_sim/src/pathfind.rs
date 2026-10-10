//! A* on the tile grid, plus a route search over the location warp graph.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

use crate::TilePos;

/// Find a 4-connected path from `start` to `goal` (inclusive of goal,
/// exclusive of start). `passable(p)` decides walkability; the goal is
/// accepted even if it isn't passable (e.g. a door or bed).
pub fn astar(start: TilePos, goal: TilePos, limit: usize, passable: impl Fn(TilePos) -> bool) -> Option<Vec<TilePos>> {
    if start == goal {
        return Some(vec![]);
    }
    let mut open = BinaryHeap::new();
    let mut came: HashMap<TilePos, TilePos> = HashMap::new();
    let mut g: HashMap<TilePos, i32> = HashMap::new();
    g.insert(start, 0);
    open.push(Reverse((start.manhattan(goal), 0, start)));
    let mut visited = 0;
    while let Some(Reverse((_, cost, p))) = open.pop() {
        if p == goal {
            let mut path = vec![p];
            let mut cur = p;
            while let Some(&prev) = came.get(&cur) {
                if prev == start {
                    break;
                }
                path.push(prev);
                cur = prev;
            }
            path.reverse();
            return Some(path);
        }
        if cost > *g.get(&p).unwrap_or(&i32::MAX) {
            continue;
        }
        visited += 1;
        if visited > limit {
            return None;
        }
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let n = TilePos::new(p.x + dx, p.y + dy);
            if n != goal && !passable(n) {
                continue;
            }
            let ng = cost + 1;
            if ng < *g.get(&n).unwrap_or(&i32::MAX) {
                g.insert(n, ng);
                came.insert(n, p);
                open.push(Reverse((ng + n.manhattan(goal), ng, n)));
            }
        }
    }
    None
}

/// Shortest sequence of locations from `from` to `to` using `edges`
/// (location -> neighbours). Includes both ends.
pub fn location_route(from: &str, to: &str, edges: &HashMap<String, Vec<String>>) -> Option<Vec<String>> {
    if from == to {
        return Some(vec![from.to_string()]);
    }
    let mut q = VecDeque::new();
    let mut prev: HashMap<String, String> = HashMap::new();
    let mut seen = HashSet::new();
    q.push_back(from.to_string());
    seen.insert(from.to_string());
    while let Some(cur) = q.pop_front() {
        for n in edges.get(&cur).into_iter().flatten() {
            if seen.insert(n.clone()) {
                prev.insert(n.clone(), cur.clone());
                if n == to {
                    let mut route = vec![to.to_string()];
                    let mut c = to.to_string();
                    while let Some(p) = prev.get(&c) {
                        route.push(p.clone());
                        c = p.clone();
                    }
                    route.reverse();
                    return Some(route);
                }
                q.push_back(n.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_path_around_wall() {
        let wall: HashSet<TilePos> = (0..5).map(|y| TilePos::new(2, y)).collect();
        let p = astar(TilePos::new(0, 0), TilePos::new(4, 0), 1000, |t| t.x >= 0 && t.y >= 0 && t.x < 6 && t.y < 7 && !wall.contains(&t)).unwrap();
        assert_eq!(*p.last().unwrap(), TilePos::new(4, 0));
        assert_eq!(p.len(), 14);
        for w in p.windows(2) {
            assert_eq!(w[0].manhattan(w[1]), 1);
        }
    }

    #[test]
    fn routes_locations() {
        let mut e = HashMap::new();
        e.insert("A".to_string(), vec!["B".to_string()]);
        e.insert("B".to_string(), vec!["A".to_string(), "C".to_string()]);
        assert_eq!(location_route("A", "C", &e).unwrap(), vec!["A", "B", "C"]);
        assert!(location_route("C", "A", &e).is_none());
    }
}
