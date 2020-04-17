#![allow(dead_code)]

use crate::error::AutomergeError;
use im_rc::HashMap;
use rand::rngs::ThreadRng;
use rand::Rng;
use std::cmp::{max, min};
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::Iterator;
use std::ops::AddAssign;

#[derive(Debug, Clone, PartialEq)]
struct Node<K>
where
    K: Clone + Debug + PartialEq,
{
    next: Vec<Link<K>>,
    prev: Vec<Link<K>>,
    level: usize,
    is_head: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Link<K>
where
    K: Clone + Debug + PartialEq,
{
    key: Option<K>,
    count: usize,
}

impl<K> AddAssign for Link<K>
where
    K: Clone + Debug + PartialEq,
{
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            key: other.key.clone(),
            count: self.count + other.count,
        };
    }
}

impl<K> Node<K>
where
    K: Debug + Clone + PartialEq,
{
    fn successor(&self) -> &Option<K> {
        if self.next.is_empty() {
            &None
        } else {
            &self.next[0].key //.as_ref()
        }
    }

    fn remove_after(&mut self, from_level: usize, removed_level: usize, links: &[Link<K>]) {
        for (level, item) in links.iter().enumerate().take(self.level).skip(from_level) {
            if level < removed_level {
                self.next[level] = item.clone();
            } else {
                self.next[level].count -= 1;
            }
        }
    }

    fn remove_before(&mut self, from_level: usize, removed_level: usize, links: &[Link<K>]) {
        for (level, item) in links.iter().enumerate().take(self.level).skip(from_level) {
            if level < removed_level {
                self.prev[level] = item.clone();
            } else {
                self.prev[level].count -= 1;
            }
        }
    }

    fn insert_after(
        &mut self,
        new_key: &K,
        new_level: usize,
        from_level: usize,
        distance: usize,
    ) -> Result<(), AutomergeError> {
        if new_level > self.level && !self.is_head {
            Err(AutomergeError::SkipListError(
                "Cannot increase the level of a non-head node".to_string(),
            ))
        } else {
            self.level = max(self.level, new_level);

            for level in from_level..self.level {
                if level < new_level {
                    let link = Link {
                        key: Some(new_key.clone()),
                        count: distance,
                    };
                    if self.next.len() == level {
                        self.next.push(link)
                    } else {
                        self.next[level] = link
                    }
                } else {
                    self.next[level].count += 1;
                }
            }

            Ok(())
        }
    }

    fn insert_before(
        &mut self,
        new_key: &K,
        new_level: usize,
        from_level: usize,
        distance: usize,
    ) -> Result<(), AutomergeError> {
        if new_level > self.level {
            Err(AutomergeError::SkipListError(
                "Cannot increase the level on insert-before".to_string(),
            ))
        } else {
            for level in from_level..self.level {
                if level < new_level {
                    self.prev[level] = Link {
                        key: Some(new_key.clone()),
                        count: distance,
                    };
                } else {
                    self.prev[level].count += 1;
                }
            }
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VecOrderedSet<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    keys: Vec<K>,
}

impl<K> VecOrderedSet<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    pub fn new() -> VecOrderedSet<K> {
        VecOrderedSet {
            keys: Vec::new(),
        }
    }
}

pub trait OrderedSet<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    fn index_of(&self, key: &K) -> Option<usize>;
    fn remove_key(&mut self, key: &K) -> Option<usize>;
    fn insert_index(&mut self, index: usize, key: K) -> Option<&K>;
    fn remove_index(&mut self, index: usize) -> Option<K>;
    fn key_of(&self, index: usize) -> Option<&K>;
}

impl<K> OrderedSet<K> for SkipList<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    fn remove_index(&mut self, index: usize) -> Option<K> {
        if let Some(key) = self.key_of(index).cloned() {
            self._remove_key(&key).ok().map(|()| key)
        } else {
            None
        }
    }

    fn remove_key(&mut self, key: &K) -> Option<usize> {
        let index = self.index_of(key);
        if index.is_some() {
            self._remove_key(key).unwrap();
        }
        index
    }

    fn key_of(&self, index: usize) -> Option<&K> {
        if index >= self.len {
            return None;
        }
        let target = index + 1;
        let mut node = &self.head;
        let mut level = node.level - 1;
        let mut count = 0;
        loop {
            while count + node.next[level].count > target {
                level -= 1
            }
            count += node.next[level].count;
            let k: &Option<K> = &node.next[level].key;
            if count == target {
                return k.as_ref();
            }
            node = self.get_node(k).unwrap(); // panic is correct
        }
    }

    fn index_of(&self, key: &K) -> Option<usize> {
        if !self.nodes.contains_key(&key) {
            return None;
        }

        let mut count = 0;
        let mut k = key.clone();
        loop {
            if let Some(node) = self.nodes.get(&k) {
                let link = &node.prev[node.level - 1];
                count += link.count;
                if let Some(key) = &link.key {
                    k = key.clone();
                } else {
                    break;
                }
            } else {
                return None;
            }
        }
        Some(count - 1)
    }

    fn insert_index(&mut self, index: usize, key: K) -> Option<&K> {
        if index == 0 {
            self.insert_head(key).unwrap();
            None
        } else {
            let suc = self.key_of(index - 1).unwrap().clone();
            self.insert_after(&suc, key).unwrap();
            self.key_of(index - 1) // FIXME
        }
    }
}

impl<K> OrderedSet<K> for VecOrderedSet<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    fn remove_index(&mut self, index: usize) -> Option<K> {
        if self.keys.len() > index {
            let k = self.keys.remove(index);
            Some(k)
        } else {
            None
        }
    }

    fn key_of(&self, index: usize) -> Option<&K> {
        self.keys.get(index)
    }

    fn index_of(&self, key: &K) -> Option<usize> {
        self.keys.iter().position(|o| o == key)
    }

    fn insert_index(&mut self, index: usize, key: K) -> Option<&K> {
        self.keys.insert(index, key);
        if index == 0 {
            None
        } else {
            self.keys.get(index - 1)
        }
    }

    fn remove_key(&mut self, key: &K) -> Option<usize> {
        if let Some(index) = self.keys.iter().position(|o| o == key) {
            self.keys.remove(index);
            Some(index)
        } else {
            None
        }
    }
}

impl<K> Default for SkipList<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K> Default for VecOrderedSet<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, K> IntoIterator for &'a VecOrderedSet<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    type Item = &'a K;
    type IntoIter = std::slice::Iter<'a, K>;

    fn into_iter(self) -> std::slice::Iter<'a, K> {
        self.keys.as_slice().iter()
    }
}

impl<'a, K> IntoIterator for &'a SkipList<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    type Item = &'a K;
    type IntoIter = SkipKeyIterator<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        SkipKeyIterator {
            id: self.head.successor(),
            nodes: &self.nodes,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SkipList<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    nodes: HashMap<K, Node<K>>,
    head: Node<K>,
    rng: ThreadRng,
    pub len: usize,
}

impl<K> PartialEq for SkipList<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.nodes.eq(&other.nodes)
    }
}

impl<K> SkipList<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    pub fn new() -> SkipList<K> {
        let nodes = HashMap::new();
        let head = Node {
            next: Vec::new(),
            prev: Vec::new(),
            level: 1,
            is_head: true,
        };
        let len = 0;
        let rng = rand::thread_rng();
        SkipList {
            nodes,
            head,
            len,
            rng,
        }
    }

    fn _remove_key(&mut self, key: &K) -> Result<(), AutomergeError> {
        let removed = self.nodes.remove(key).ok_or_else(|| {
            AutomergeError::SkipListError(
                "The given key cannot be removed because it does not exist".to_string(),
            )
        })?;
        let max_level = self.head.level;
        let mut pre = self.predecessors(&removed.prev[0].key, max_level)?;
        let mut suc = self.successors(&removed.next[0].key, max_level)?;

        for i in 0..max_level {
            let distance = pre[i].count + suc[i].count - 1;
            pre[i].count = distance;
            suc[i].count = distance;
        }

        self.len -= 1;
        let mut pre_level = 0;
        let mut suc_level = 0;

        for level in 1..(max_level + 1) {
            let update_level = min(level, removed.level);
            if level == max_level
                || pre.get(level).map(|l| &l.key) != pre.get(pre_level).map(|l| &l.key)
            {
                self.get_node_mut(&pre[pre_level].key)?.remove_after(
                    pre_level,
                    update_level,
                    &suc,
                );
                pre_level = level;
            }
            if suc[suc_level].key.is_some()
                && (level == max_level
                    || suc.get(level).map(|l| &l.key) != suc.get(suc_level).map(|l| &l.key))
            {
                self.get_node_mut(&suc[suc_level].key)?.remove_before(
                    suc_level,
                    update_level,
                    &pre,
                );
                suc_level = level;
            }
        }
        Ok(())
    }

    fn get_node(&self, key: &Option<K>) -> Result<&Node<K>, AutomergeError> {
        if let Some(ref k) = key {
            self.nodes
                .get(k)
                .ok_or_else(|| AutomergeError::SkipListError("Key not found".to_string()))
        } else {
            Ok(&self.head)
        }
    }

    fn get_node_mut(&mut self, key: &Option<K>) -> Result<&mut Node<K>, AutomergeError> {
        if let Some(ref k) = key {
            self.nodes
                .get_mut(k)
                .ok_or_else(|| AutomergeError::SkipListError("Key not found".to_string()))
        } else {
            Ok(&mut self.head)
        }
    }

    fn predecessors(
        &self,
        predecessor: &Option<K>,
        max_level: usize,
    ) -> Result<Vec<Link<K>>, AutomergeError> {
        let mut pre = vec![Link {
            key: predecessor.clone(),
            count: 1,
        }];

        for level in 1..max_level {
            let mut link = pre[level - 1].clone();
            while link.key.is_some() {
                let node = self.get_node(&link.key)?;
                if node.level > level {
                    break;
                }
                if node.level < level {
                    return Err(AutomergeError::SkipListError(
                        "Level lower than expected".to_string(),
                    ));
                }
                link += node.prev[level - 1].clone();
            }
            pre.push(link);
        }
        Ok(pre)
    }

    fn successors(
        &self,
        successor: &Option<K>,
        max_level: usize,
    ) -> Result<Vec<Link<K>>, AutomergeError> {
        let mut suc = vec![Link {
            key: successor.clone(),
            count: 1,
        }];

        for level in 1..max_level {
            let mut link = suc[level - 1].clone();
            while link.key.is_some() {
                let node = self.get_node(&link.key)?;
                if node.level > level {
                    break;
                }
                if node.level < level {
                    return Err(AutomergeError::SkipListError(
                        "Level lower than expected".to_string(),
                    ));
                }
                link += node.next[level - 1].clone();
            }
            suc.push(link);
        }
        Ok(suc)
    }

    pub fn insert_head(&mut self, key: K) -> Result<(), AutomergeError> {
        self._insert_after(&None, key)
    }

    pub fn insert_after(
        &mut self,
        predecessor: &K,
        key: K,
    ) -> Result<(), AutomergeError> {
        self._insert_after(&Some(predecessor.clone()), key)
    }

    fn _insert_after(
        &mut self,
        predecessor: &Option<K>,
        key: K,
    ) -> Result<(), AutomergeError> {
        if self.nodes.contains_key(&key) {
            return Err(AutomergeError::SkipListError("DuplicateKey".to_string()));
        }

        let new_level = self.random_level();
        let max_level = max(new_level, self.head.level);
        let successor = self.get_node(predecessor)?.successor();
        let mut pre = self.predecessors(predecessor, max_level)?;
        let mut suc = self.successors(successor, max_level)?;

        self.len += 1;

        let mut pre_level = 0;
        let mut suc_level = 0;
        for level in 1..(max_level + 1) {
            let update_level = min(level, new_level);
            if level == max_level
                || pre.get(level).map(|l| &l.key) != pre.get(pre_level).map(|l| &l.key)
            {
                self.get_node_mut(&pre[pre_level].key)?.insert_after(
                    &key,
                    update_level,
                    pre_level,
                    pre[pre_level].count,
                )?;
                pre_level = level;
            }
            if suc[suc_level].key.is_some()
                && (level == max_level
                    || suc.get(level).map(|l| &l.key) != suc.get(suc_level).map(|l| &l.key))
            {
                self.get_node_mut(&suc[suc_level].key)?.insert_before(
                    &key,
                    update_level,
                    suc_level,
                    suc[suc_level].count,
                )?;
                suc_level = level;
            }
        }

        pre.truncate(new_level);
        suc.truncate(new_level);
        self.nodes.insert(
            key,
            Node {
                    level: new_level,
                    prev: pre,
                    next: suc,
                    is_head: false,
            },
        );
        Ok(())
    }

    // Returns a random number from the geometric distribution with p = 0.75.
    // That is, returns k with probability p * (1 - p)^(k - 1).
    // For example, returns 1 with probability 3/4, returns 2 with probability 3/16,
    // returns 3 with probability 3/64, and so on.

    fn random_level(&mut self) -> usize {
        // Create random number between 0 and 2^32 - 1
        // Count leading zeros in that 32-bit number
        let rand: u32 = self.rng.gen();
        let mut level = 1;
        while rand < 1 << (32 - 2 * level) && level < 16 {
            level += 1
        }
        level
    }
}

pub(crate) struct SkipKeyIterator<'a, K>
where
    K: Debug + Clone + PartialEq,
{
    id: &'a Option<K>,
    nodes: &'a HashMap<K, Node<K>>,
}

impl<'a, K> Iterator for SkipKeyIterator<'a, K>
where
    K: Debug + Clone + Hash + PartialEq + Eq,
{
    type Item = &'a K;

    fn next(&mut self) -> Option<&'a K> {
        match &self.id {
            None => None,
            Some(ref key) => {
                if let Some(ref node) = &self.nodes.get(key) {
                    self.id = node.successor();
                    Some(key)
                } else {
                    panic!("iter::next hit a dead end")
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Delta<K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    index: isize,
    key: Option<K>,
}

// this is an experiment to if I can change request processing
// index lookups by not mutating the skip list
// throuput was quite signifigant actually - about 1.5x over in the
// mass edit perf test
// ideally we can speed up the skip list enough to not need this
// also this could perform worse if the ops per change were huge
// eg.. 10,000 changes with 10 ops each vs 10 changes with 10,000 ops each

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OrdDelta<'a, K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    list: Option<&'a SkipList<K>>,
    delta: Vec<Delta<K>>,
}

impl<'a, K> OrdDelta<'a, K>
where
    K: Clone + Debug + Hash + PartialEq + Eq,
{
    pub fn new(list: Option<&'a SkipList<K>>) -> OrdDelta<'a, K> {
        OrdDelta {
            list,
            delta: Vec::new(),
        }
    }

    pub fn insert_index(&mut self, index: usize, key: K) {
        let index = index as isize;
        let delta = Delta {
            index,
            key: Some(key),
        };
        for i in 0..self.delta.len() {
            if self.delta[i].index >= index {
                self.delta.iter_mut().skip(i).for_each(|d| d.index += 1);
                self.delta.insert(i, delta);
                return;
            }
        }
        self.delta.push(delta);
    }

    pub fn key_of(&self, index: usize) -> Option<K> {
        let index = index as isize;
        let mut acc: isize = 0;
        for i in 0..self.delta.len() {
            match &self.delta[i] {
                Delta {
                    index: j,
                    key: Some(key),
                } => {
                    if j == &index {
                        return Some(key.clone());
                    }
                    if j > &index {
                        break;
                    }
                    acc += 1;
                }
                Delta {
                    index: j,
                    key: None,
                } => {
                    if j > &index {
                        break;
                    }
                    acc -= 1;
                }
            }
        }
        self.list
            .and_then(|l| l.key_of((index as isize - acc) as usize).cloned())
    }

    pub fn remove_index(&mut self, index: usize) -> Option<K> {
        let index = index as isize;
        let delta = Delta { index, key: None };
        for i in 0..self.delta.len() {
            if self.delta[i].index == index && self.delta[i].key.is_some() {
                let old_insert = self.delta.remove(i);
                self.delta.iter_mut().skip(i).for_each(|d| d.index -= 1);
                return old_insert.key;
            }
            if self.delta[i].index > index {
                let key = self.key_of(index as usize);
                self.delta.iter_mut().skip(i).for_each(|d| d.index -= 1);
                self.delta.insert(i, delta);
                return key;
            }
        }
        let key = self.key_of(index as usize);
        self.delta.push(delta);
        key
    }
}

// get(n)
// insert(n)
// len()
// remove(n)
// get_index_for(T)
// insert_after_(i,K,V)

#[cfg(test)]
mod tests {
    use super::*;
    //use std::str::FromStr;

    #[test]
    fn test_index_of() -> Result<(), AutomergeError> {
        let mut s = SkipList::<&str>::new();

        // should return None on an empty list
        assert_eq!(s.index_of(&"foo"), None);

        // should return None for a nonexistent key
        s.insert_head("foo")?;
        assert_eq!(s.index_of(&"baz"), None);

        // should return 0 for the first list element
        assert_eq!(s.index_of(&"foo"), Some(0));

        // should return length-1 for the last list element
        s.insert_after(&"foo", "bar")?;
        s.insert_after(&"bar", "baz")?;
        assert_eq!(s.index_of(&"baz"), Some(s.len - 1));

        // should adjust based on removed elements
        s.remove_key(&"foo");
        assert_eq!(s.index_of(&"bar"), Some(0));
        assert_eq!(s.index_of(&"baz"), Some(1));
        s.remove_key(&"bar");
        assert_eq!(s.index_of(&"baz"), Some(0));
        Ok(())
    }

    #[test]
    fn test_len() -> Result<(), AutomergeError> {
        let mut s = SkipList::<&str>::new();

        //should be 0 for an empty list
        assert_eq!(s.len, 0);

        // should increase by 1 for every insertion
        s.insert_head("a3")?;
        s.insert_head("a2")?;
        s.insert_head("a1")?;
        assert_eq!(s.len, 3);

        //should decrease by 1 for every removal
        s.remove_key(&"a2");
        assert_eq!(s.len, 2);
        Ok(())
    }

    #[test]
    fn test_key_of() -> Result<(), AutomergeError> {
        let mut s = SkipList::<&str>::new();

        // should return None on an empty list
        assert_eq!(s.key_of(0), None);

        // should return None for an index past the end of the list
        s.insert_head("a3")?;
        s.insert_head("a2")?;
        s.insert_head("a1")?;
        assert_eq!(s.key_of(10), None);

        // should return the first key for index 0
        assert_eq!(s.key_of(0), Some(&"a1"));

        // should return the last key for index -1
        // assert_eq!(s.key_of(-1), Some("a3"));

        // should return the last key for index length-1
        assert_eq!(s.key_of(s.len - 1), Some(&"a3"));

        // should not count removed elements
        s.remove_key(&"a1");
        s.remove_key(&"a3");
        assert_eq!(s.key_of(0), Some(&"a2"));

        Ok(())
    }

    #[test]
    fn test_insert_index() -> Result<(), AutomergeError> {
        let mut s = SkipList::<&str>::new();

        // should insert the new key-value pair at the given index
        s.insert_head("aaa")?;
        s.insert_after(&"aaa", "ccc")?;
        s.insert_index(1, "bbb");
        assert_eq!(s.index_of(&"aaa"), Some(0));
        assert_eq!(s.index_of(&"bbb"), Some(1));
        assert_eq!(s.index_of(&"ccc"), Some(2));

        // should insert at the head if the index is zero
        s.insert_index(0, "a");
        assert_eq!(s.key_of(0), Some(&"a"));
        Ok(())
    }

    #[test]
    fn test_remove_index() -> Result<(), AutomergeError> {
        let mut s = SkipList::<&str>::new();

        // should remove the value at the given index
        s.insert_head("ccc")?;
        s.insert_head("bbb")?;
        s.insert_head("aaa")?;
        s.remove_index(1);
        assert_eq!(s.index_of(&"aaa"), Some(0));
        assert_eq!(s.index_of(&"bbb"), None);
        assert_eq!(s.index_of(&"ccc"), Some(1));

        // should raise an error if the given index is out of bounds
        assert_eq!(s.remove_index(100), None);
        Ok(())
    }

    #[test]
    fn test_remove_key_big() -> Result<(), AutomergeError> {
        let mut s = SkipList::<String>::new();
        for i in 0..10000 {
            let j = 9999 - i;
            s.insert_head(format!("a{}", j))?;
        }

        assert_eq!(s.index_of(&"a20".to_string()), Some(20));
        assert_eq!(s.index_of(&"a500".to_string()), Some(500));
        assert_eq!(s.index_of(&"a1000".to_string()), Some(1000));

        for i in 0..5000 {
            let j = (4999 - i) * 2 + 1;
            s.remove_index(j);
        }

        assert_eq!(s.index_of(&"a4000".to_string()), Some(2000));
        assert_eq!(s.index_of(&"a1000".to_string()), Some(500));
        assert_eq!(s.index_of(&"a500".to_string()), Some(250));
        assert_eq!(s.index_of(&"a20".to_string()), Some(10));

        Ok(())
    }

    #[test]
    fn test_remove_key() -> Result<(), AutomergeError> {
        let mut s = SkipList::<&str>::new();
        s.insert_head("a20")?;
        s.insert_head("a19")?;
        s.insert_head("a18")?;
        s.insert_head("a17")?;
        s.insert_head("a16")?;
        s.insert_head("a15")?;
        s.insert_head("a14")?;
        s.insert_head("a13")?;
        s.insert_head("a12")?;
        s.insert_head("a11")?;
        s.insert_head("a10")?;
        s.insert_head("a9")?;
        s.insert_head("a8")?;
        s.insert_head("a7")?;
        s.insert_head("a6")?;
        s.insert_head("a5")?;
        s.insert_head("a4")?;
        s.insert_head("a3")?;
        s.insert_head("a2")?;
        s.insert_head("a1")?;
        s.insert_head("a0")?;

        assert_eq!(s.index_of(&"a20"), Some(20));

        s.remove_key(&"a1");
        s.remove_key(&"a3");
        s.remove_key(&"a5");
        s.remove_key(&"a7");
        s.remove_key(&"a9");
        s.remove_key(&"a11");
        s.remove_key(&"a13");
        s.remove_key(&"a15");
        s.remove_key(&"a17");
        s.remove_key(&"a19");

        assert_eq!(s.index_of(&"a20"), Some(10));
        assert_eq!(s.index_of(&"a10"), Some(5));
        Ok(())
    }
}
