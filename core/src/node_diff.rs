use std::collections::HashMap;

pub struct CNode {
    pub lines: Vec<String>,
    // pub text: String,
    // pub lines: usize,
}

impl CNode {
    fn new(text: &str) -> Self {
        CNode { lines: text.lines().map(|l| l.to_string()).collect() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assignment {
    OrignalID(usize),
    NewNode,
}

type Cost = usize;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Choice {
    Same,
    Dropped,
    Added,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Res(Cost, Choice);

struct State<'a> {
    orig: &'a [CNode],
    existing_assignment: &'a [usize],
    new: &'a [CNode],

    node_choices: HashMap<(usize, usize), Res>,
    change_costs: HashMap<(usize, usize), Cost>,
}

pub fn compare(orig: &[CNode], existing_assignment: &[usize], new: &[CNode]) -> Vec<Assignment> {
    let mut state = State {
        orig,
        existing_assignment,
        new,
        node_choices: HashMap::new(),
        change_costs: HashMap::new(),
    };

    compare_nodes(&mut state, 0, 0);

    let mut ass = Vec::with_capacity(new.len());
    let mut i = 0;
    let mut j = 0;
    println!("choices: {:#?}", state.node_choices);
    loop {
        let choice = state.node_choices.get(&(i, j)).expect(&format!("missing choice for {},{}", i, j));

        match choice.1 {
            Choice::Added => {
                ass.push(Assignment::NewNode);
                j += 1;
            },
            Choice::Dropped => {
                i += 1;
            },
            Choice::Same => {
                ass.push(Assignment::OrignalID(existing_assignment[i]));
                i += 1;
                j += 1;
            },
        }

        if i == state.orig.len() && j == state.new.len() {
            return ass;
        }
    }
}

fn compare_nodes(state: &mut State, i: usize, j: usize) -> Cost {
    if i == state.orig.len() && j == state.new.len() {
        return 0;
    }

    if let Some(c) = state.node_choices.get(&(i, j)) {
        return c.0;
    }

    let mut c = Res(Cost::MAX, Choice::Same);

    if i < state.orig.len() && j < state.new.len() {
        let k = *state.change_costs
            .entry((i, j))
            .or_insert_with(|| find_change_cost(&state.orig[i], &state.new[j]));
        let same = Res(
            k.saturating_add(compare_nodes(state, i + 1, j + 1)),
            Choice::Same);
        c = c.min(same);
    }

    if j < state.new.len() {
        let added = Res(
            state.new[j].lines.len().saturating_add(compare_nodes(state, i, j + 1)),
            Choice::Added);
        c = c.min(added);
    }

    if i < state.orig.len() {
        let dropped = Res(
            state.orig[i].lines.len().saturating_add(compare_nodes(state, i + 1, j)),
            Choice::Dropped);
        c = c.min(dropped);
    }

    state.node_choices.insert((i, j), c);
    c.0
}

fn find_change_cost(a: &CNode, b: &CNode) -> Cost {
    let diff_ops = similar::capture_diff_slices(similar::Algorithm::Myers, &a.lines, &b.lines);
    diff_ops.len()
}


#[cfg(test)]
mod test {
    use super::{Assignment, CNode, compare};

    #[test]
    fn added_before() {
        let orig = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#)
        ];

        let existing_assignment = [
            100,
        ];

        let new = [
            CNode::new(r#"
                begin g()
                    baz
                end
            "#),
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
        ];


        let res = compare(&orig, &existing_assignment, &new);
        assert_eq!(res, vec![
            Assignment::NewNode,
            Assignment::OrignalID(100),
        ]);
    }

    #[test]
    fn added_after() {
        let orig = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#)
        ];

        let existing_assignment = [
            100,
        ];

        let new = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin g()
                    baz
                end
            "#),
        ];


        let res = compare(&orig, &existing_assignment, &new);
        assert_eq!(res, vec![
            Assignment::OrignalID(100),
            Assignment::NewNode,
        ]);
    }

    #[test]
    fn added_middle() {
        let orig = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin h()
                    baz
                end
            "#),
        ];

        let existing_assignment = [
            100,
            200,
        ];

        let new = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin g()
                    qux
                end
            "#),
            CNode::new(r#"
                begin h()
                    baz
                end
            "#),
        ];


        let res = compare(&orig, &existing_assignment, &new);
        assert_eq!(res, vec![
            Assignment::OrignalID(100),
            Assignment::NewNode,
            Assignment::OrignalID(200),
        ]);
    }

    #[test]
    fn dropped_start() {
        let orig = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin g()
                    qux
                end
            "#),
            CNode::new(r#"
                begin h()
                    baz
                end
            "#),
        ];

        let existing_assignment = [
            100,
            150,
            200,
        ];

        let new = [
            CNode::new(r#"
                begin g()
                    qux
                end
            "#),
            CNode::new(r#"
                begin h()
                    baz
                end
            "#),
        ];


        let res = compare(&orig, &existing_assignment, &new);
        assert_eq!(res, vec![
            Assignment::OrignalID(150),
            Assignment::OrignalID(200),
        ]);
    }

    #[test]
    fn dropped_end() {
        let orig = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin g()
                    qux
                end
            "#),
            CNode::new(r#"
                begin h()
                    baz
                end
            "#),
        ];

        let existing_assignment = [
            100,
            150,
            200,
        ];

        let new = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin g()
                    qux
                end
            "#),
        ];


        let res = compare(&orig, &existing_assignment, &new);
        assert_eq!(res, vec![
            Assignment::OrignalID(100),
            Assignment::OrignalID(150),
        ]);
    }

    #[test]
    fn dropped_middle() {
        let orig = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin g()
                    qux
                end
            "#),
            CNode::new(r#"
                begin h()
                    baz
                end
            "#),
        ];

        let existing_assignment = [
            100,
            150,
            200,
        ];

        let new = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin h()
                    baz
                end
            "#),
        ];


        let res = compare(&orig, &existing_assignment, &new);
        assert_eq!(res, vec![
            Assignment::OrignalID(100),
            Assignment::OrignalID(200),
        ]);
    }

    #[test]
    fn changed() {
        let orig = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin g()
                    qux
                end
            "#),
            CNode::new(r#"
                begin h()
                    baz
                end
            "#),
        ];

        let existing_assignment = [
            100,
            150,
            200,
        ];

        let new = [
            CNode::new(r#"
                begin f()
                    foo
                    bar
                end
            "#),
            CNode::new(r#"
                begin g()
                    qux
                    added lime
                end
            "#),
            CNode::new(r#"
                begin h()
                    baz
                end
            "#),
        ];


        let res = compare(&orig, &existing_assignment, &new);
        assert_eq!(res, vec![
            Assignment::OrignalID(100),
            Assignment::OrignalID(150),
            Assignment::OrignalID(200),
        ]);
    }
}
