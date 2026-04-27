use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleInfo {
    pub task_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockedInfo {
    pub depends_on: Vec<String>,
    pub blocked_by: Vec<String>,
    pub missing_dependencies: Vec<String>,
    pub in_cycle: bool,
    pub ready: bool,
}

pub fn detect_cycles(tasks: &[(String, Vec<String>)]) -> Vec<CycleInfo> {
    let task_ids: HashSet<_> = tasks.iter().map(|(id, _)| id.clone()).collect();
    let dependencies: HashMap<_, _> = tasks.iter().cloned().collect();
    let mut indegree: HashMap<String, usize> = task_ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

    for (task_id, deps) in tasks {
        for dep in deps.iter().filter(|dep| task_ids.contains(*dep)) {
            if let Some(count) = indegree.get_mut(task_id) {
                *count += 1;
            }
            dependents
                .entry(dep.clone())
                .or_default()
                .push(task_id.clone());
        }
    }

    let mut queue: VecDeque<String> = indegree
        .iter()
        .filter_map(|(id, count)| (*count == 0).then(|| id.clone()))
        .collect();
    let mut processed = HashSet::new();

    while let Some(task_id) = queue.pop_front() {
        processed.insert(task_id.clone());
        if let Some(children) = dependents.get(&task_id) {
            for child in children {
                if let Some(count) = indegree.get_mut(child) {
                    *count = count.saturating_sub(1);
                    if *count == 0 {
                        queue.push_back(child.clone());
                    }
                }
            }
        }
    }

    let remaining: HashSet<String> = task_ids.difference(&processed).cloned().collect();
    let mut cycle_members: Vec<_> = remaining
        .iter()
        .filter(|id| reaches_self(id, &dependencies, &remaining))
        .cloned()
        .collect();
    cycle_members.sort();
    connected_cycle_groups(cycle_members, &dependencies)
}

pub fn compute_blocked(
    task_id: &str,
    depends_on: &[String],
    all_tasks: &HashMap<String, &str>,
    cycle_members: &HashSet<String>,
) -> BlockedInfo {
    let mut blocked_by = Vec::new();
    let mut missing_dependencies = Vec::new();

    for dep in depends_on {
        match all_tasks.get(dep.as_str()) {
            Some(location) if *location == "done" => {}
            Some(_) => blocked_by.push(dep.clone()),
            None => {
                missing_dependencies.push(dep.clone());
                blocked_by.push(dep.clone());
            }
        }
    }

    let in_cycle = cycle_members.contains(task_id);
    let ready = all_tasks.get(task_id) == Some(&"todo") && blocked_by.is_empty() && !in_cycle;
    BlockedInfo {
        depends_on: depends_on.to_vec(),
        blocked_by,
        missing_dependencies,
        in_cycle,
        ready,
    }
}

fn reaches_self(
    start: &str,
    dependencies: &HashMap<String, Vec<String>>,
    remaining: &HashSet<String>,
) -> bool {
    let mut stack = dependencies.get(start).cloned().unwrap_or_default();
    let mut seen = HashSet::new();
    while let Some(next) = stack.pop() {
        if next == start {
            return true;
        }
        if !remaining.contains(&next) || !seen.insert(next.clone()) {
            continue;
        }
        if let Some(deps) = dependencies.get(&next) {
            stack.extend(deps.iter().cloned());
        }
    }
    false
}

fn connected_cycle_groups(
    cycle_members: Vec<String>,
    dependencies: &HashMap<String, Vec<String>>,
) -> Vec<CycleInfo> {
    let cycle_set: HashSet<_> = cycle_members.iter().cloned().collect();
    let mut visited = HashSet::new();
    let mut groups = Vec::new();

    for member in cycle_members {
        if !visited.insert(member.clone()) {
            continue;
        }
        let mut group = vec![member.clone()];
        let mut stack = vec![member];
        while let Some(current) = stack.pop() {
            for neighbor in cycle_neighbors(&current, dependencies, &cycle_set) {
                if visited.insert(neighbor.clone()) {
                    group.push(neighbor.clone());
                    stack.push(neighbor);
                }
            }
        }
        group.sort();
        groups.push(CycleInfo { task_ids: group });
    }
    groups
}

fn cycle_neighbors(
    task_id: &str,
    dependencies: &HashMap<String, Vec<String>>,
    cycle_set: &HashSet<String>,
) -> Vec<String> {
    let mut neighbors: Vec<String> = dependencies
        .get(task_id)
        .into_iter()
        .flat_map(|deps| deps.iter())
        .filter(|dep| cycle_set.contains(*dep))
        .cloned()
        .collect();
    neighbors.extend(
        dependencies
            .iter()
            .filter(|(_, deps)| deps.iter().any(|dep| dep == task_id))
            .map(|(id, _)| id)
            .filter(|id| cycle_set.contains(*id))
            .cloned(),
    );
    neighbors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_cycles_and_ignores_acyclic_graphs() {
        assert!(detect_cycles(&[("A".into(), vec!["B".into()])]).is_empty());
        assert_eq!(
            detect_cycles(&[
                ("A".into(), vec!["B".into()]),
                ("B".into(), vec!["A".into()])
            ]),
            vec![CycleInfo {
                task_ids: vec!["A".into(), "B".into()]
            }]
        );
        assert_eq!(
            detect_cycles(&[
                ("A".into(), vec!["B".into()]),
                ("B".into(), vec!["C".into()]),
                ("C".into(), vec!["A".into()]),
                ("D".into(), vec!["E".into()])
            ])[0]
                .task_ids,
            vec!["A", "B", "C"]
        );
        assert_eq!(
            detect_cycles(&[("A".into(), vec!["A".into()])]),
            vec![CycleInfo {
                task_ids: vec!["A".into()]
            }]
        );
    }

    #[test]
    fn detects_independent_cycle_subgraphs() {
        let cycles = detect_cycles(&[
            ("A".into(), vec!["B".into()]),
            ("B".into(), vec!["A".into()]),
            ("C".into(), vec!["D".into()]),
            ("D".into(), vec!["C".into()]),
        ]);
        assert_eq!(cycles.len(), 2);
        assert!(cycles.iter().any(|cycle| cycle.task_ids == ["A", "B"]));
        assert!(cycles.iter().any(|cycle| cycle.task_ids == ["C", "D"]));
    }

    #[test]
    fn computes_blocked_status() {
        let all_tasks = HashMap::from([("A".into(), "todo"), ("B".into(), "done")]);
        let empty_cycles = HashSet::new();
        let satisfied = compute_blocked("A", &["B".into()], &all_tasks, &empty_cycles);
        assert!(satisfied.ready);
        assert!(satisfied.blocked_by.is_empty());

        let unmet = compute_blocked("A", &["C".into()], &all_tasks, &empty_cycles);
        assert_eq!(unmet.blocked_by, ["C"]);
        assert_eq!(unmet.missing_dependencies, ["C"]);
        assert!(!unmet.ready);

        let cycle_members = HashSet::from(["A".into()]);
        let in_cycle = compute_blocked("A", &[], &all_tasks, &cycle_members);
        assert!(in_cycle.in_cycle);
        assert!(!in_cycle.ready);
    }
}
