use std::{collections::HashSet, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Scope {
    pub project_root: PathBuf,
    pub source_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct Aggregator {
    pub source_dependencies: HashSet<Dependency>,
    pub static_dependencies: HashSet<Dependency>,
    pub implicit_dependencies: HashSet<Dependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dependency {
    pub origin: PathBuf,
    pub target: PathBuf,
    pub is_internal: Option<bool>,
}

impl Scope {
    pub fn source_dir(&self) -> PathBuf {
        self.source_path.parent().unwrap().to_path_buf()
    }
}

impl Aggregator {
    pub fn wrap<Value>(self, value: Value) -> State<Value> {
        State { aggregator: self, value }
    }
    pub fn union(left: Self, right: Self) -> Self {
        Aggregator {
            source_dependencies: left.source_dependencies.union(&right.source_dependencies).cloned().collect(),
            static_dependencies: left.static_dependencies.union(&right.static_dependencies).cloned().collect(),
            implicit_dependencies: left.implicit_dependencies.union(&right.implicit_dependencies).cloned().collect(),
        }
    }
    pub fn merge(self, other: Self) -> Self {
        Self::union(self, other)
    }
    pub fn include(&mut self, other: Self) {
        self.source_dependencies.extend(other.source_dependencies);
        self.static_dependencies.extend(other.static_dependencies);
    }
    pub fn flatten(items: impl IntoIterator<Item=Aggregator>) -> Aggregator {
        let initial_state = Aggregator::default();
        // let initial_state = len_hint
        //     .map(|len| {
        //         Aggregator {
        //             source_dependencies: HashSet::with_capacity(len),
        //             static_dependencies: HashSet::with_capacity(len),
        //             implicit_dependencies: HashSet::with_capacity(len),
        //         }
        //     })
        //     .unwrap_or_default();
        items
            .into_iter()
            .fold(initial_state, |mut acc, item| {
                acc.include(item);
                acc
            })
    }
}

#[derive(Debug, Clone)]
pub struct State<T> {
    pub aggregator: Aggregator,
    pub value: T,
}

impl<T> State<T> {
    pub fn map<Result>(self, apply: impl FnOnce(T) -> Result) -> State<Result> {
        State { aggregator: self.aggregator, value: apply(self.value) }
    }
    pub fn and_then<Result>(self, apply: impl FnOnce(T) -> State<Result>) -> State<Result> {
        let State { aggregator, value } = apply(self.value);
        State {
            aggregator: self.aggregator.merge(aggregator),
            value: value,
        }
    }
    pub fn map_with<Result>(self, apply: impl FnOnce(T, &mut Aggregator) -> Result) -> State<Result> {
        let mut aggregator = self.aggregator;
        let result_value = apply(self.value, &mut aggregator);
        State { aggregator: aggregator, value: result_value }
    }
    pub fn wrap(value: T) -> Self {
        Self { aggregator: Default::default(), value: value }
    }
    pub fn flatten(items: impl IntoIterator<Item=State<T>>, len_hint: Option<usize>) -> State<Vec<T>> {
        let initial_state = len_hint
            .map(|len| {
                State::<Vec<T>>::with_capacity(len)
            })
            .unwrap_or_default();
        items
            .into_iter()
            .fold(initial_state, |mut acc, item| {
                let State { aggregator, value } = item;
                acc.aggregator.include(aggregator);
                acc.value.push(value);
                acc
            })
    }
}

impl<T> Default for State<Vec<T>> {
    fn default() -> Self {
        State { aggregator: Default::default(), value: Vec::default() }
    }
}

impl<T> State<Vec<T>> {
    fn with_capacity(capacity: usize) -> Self {
        State {
            aggregator: Default::default(),
            value: Vec::with_capacity(capacity)
        }
    }
}


