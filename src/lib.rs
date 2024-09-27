//! Performs a set of batched insertions on a vector.
//!
//! [`Vec::insert(index, value)`][Vec::insert] takes `O(n)` time to move internal memory,
//! so calling it in a loop can cause quadratic blowup.
//!
//! If you batch multiple values together with an [`InsertionSet`]
//! you can defer the expensive movement of the vector's
//! memory till the of the loop.
//!
//! This code was originally copied from the first prototype compiler for [DuckLogic].
//! It was inspired by the way the [B3 JIT] handles insertions.
//!
//! [DuckLogic]: https://ducklogic.org/
//! [B3 JIT]: https://webkit.org/blog/5852/introducing-the-b3-jit-compiler/
use std::fmt::Debug;
use std::iter::{ExactSizeIterator, FromIterator};
use std::ops::Range;

use self::sorting::insertion_sort_by_key;

mod shift;
mod sorting;

use self::shift::BulkShifter;

/// A value that is pending insertion
#[derive(Debug)]
pub struct Insertion<T> {
    /// Where in the original vector to insert this value.
    ///
    /// This is equivelant to the index argument in `Vec::insert`
    pub index: usize,
    /// The value to be inserted
    pub element: T,
}
impl<T> Insertion<T> {
    /// Create a new Insertion
    #[inline]
    pub fn new(index: usize, element: T) -> Self {
        Insertion { index, element }
    }
}
impl<T> From<(usize, T)> for Insertion<T> {
    #[inline]
    fn from(tuple: (usize, T)) -> Self {
        Insertion::new(tuple.0, tuple.1)
    }
}

/// A set of pending insertions on a Vec
///
/// When multiple insertions at a
///
/// See module documentation for an overview.
pub struct InsertionSet<T> {
    insertions: Vec<Insertion<T>>,
}
impl<T> InsertionSet<T> {
    /// Create a new InsertionSet
    #[inline]
    pub fn new() -> Self {
        InsertionSet {
            insertions: Vec::new(),
        }
    }
    /// Queue the specified insertion
    ///
    /// If there are multiple insertions at the same index,
    /// they will be applied in the order queued.
    #[inline]
    pub fn push(&mut self, insertion: Insertion<T>) {
        self.insertions.push(insertion)
    }
    /// Insert the element to be inserted before the given index
    ///
    /// If multiple elements are queued to be inserted at the same index,
    /// they will be applied in the original order queued.
    #[inline]
    pub fn insert(&mut self, index: usize, element: T) {
        self.push(Insertion { index, element })
    }
    /// Apply all of the pending insertions against the specified vector,
    /// returning the result
    #[inline]
    pub fn applied(mut self, mut target: Vec<T>) -> Vec<T> {
        self.apply(&mut target);
        target
    }
    /// The number of insertions that are currently queued
    #[inline]
    pub fn desired_insertions(&self) -> usize {
        self.insertions.len()
    }
    /// List the updated locations of all the elements (both original and newly inserted).
    ///
    /// See [Self::compute_updated_locations] for details
    pub fn list_updated_locations(&mut self, target: &[T]) -> Vec<(OriginalLocation, usize)> {
        let mut result = Vec::with_capacity(target.len() + self.desired_insertions());
        self.compute_updated_locations(target, |original, updated| {
            result.push((original, updated))
        });
        result.sort_by_key(|&(_, updated)| updated);
        result
    }
    /// Compute the updated locations of all the elements (both original and newly inserted).
    ///
    /// Assumes this set of insertions are being applied against the specified slice,
    /// invoking the callback on each element (even if the location is unchanged).
    ///
    /// If any of the insertion indexes are out of bounds of the original vec,
    /// then this function will panic.
    pub fn compute_updated_locations<F>(&mut self, target: &[T], mut func: F)
    where
        F: FnMut(OriginalLocation, usize),
    {
        self.sort();
        compute_updated_locations(
            target,
            self.insertions
                .iter()
                .rev()
                .map(|insertion| insertion.index),
            |original, updated| {
                func(
                    match original {
                        OriginalLocation::Original(_) => original,
                        OriginalLocation::Insertion(reversed_index) => {
                            // Convert the reversed insertion index back to the original one
                            OriginalLocation::Insertion(
                                self.insertions.len() - (reversed_index + 1),
                            )
                        }
                    },
                    updated,
                )
            },
        )
    }
    /// Applies all the insertions to the specified target vector.
    ///
    /// This reuses the Vector's existing memory if possible,
    /// but may require a reallocation (due to new values)
    ///
    /// The average runtime of this function is `O(n + m)`,
    /// where `n` is the number of existing elements and `m` is the number of insertions.
    pub fn apply(&mut self, target: &mut Vec<T>) {
        self.sort();
        apply_bulk_insertions(target, PoppingIter(&mut self.insertions));
    }
    fn sort(&mut self) {
        /*
         * Why would we possibly want to use insertion sort here?
         * First of all,
         * we need to maintain a stable sort to preserve the original order of the `Insertion`s.
         * Insertion sort has many other advantages over mergesort and quicksort,
         * and can be significantly faster in some scenarios.
         *
         * When the array is already mostly sorted, insertion sort has average running time `O(nk)`,
         * where `k` is the average distance of each element from its proper position.
         * In a randomly sorted array `k == n` giving `O(n^2)` worst case performance,
         * this isn't true in all scenarios as `k` may be significantly smaller.
         * We expect the `InsertionSet` to be mostly sorted already,
         * with only a few slightly out of place elements,
         * giving a very low average `k` value and very good running time.
         *
         * This is inspired by WebKit's choice to use bubble sort for their insertion set,
         * except that bubble sort is a terrible algorithm and insertion sort is much better.
         */
        insertion_sort_by_key(&mut *self.insertions, |insertion| insertion.index);
    }
}
impl<T> FromIterator<Insertion<T>> for InsertionSet<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = Insertion<T>>>(iter: I) -> Self {
        InsertionSet {
            insertions: iter.into_iter().collect(),
        }
    }
}
impl<T> FromIterator<(usize, T)> for InsertionSet<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = (usize, T)>>(iter: I) -> Self {
        iter.into_iter().map(Insertion::from).collect()
    }
}
impl<T> Default for InsertionSet<T> {
    #[inline]
    fn default() -> Self {
        InsertionSet::new()
    }
}

struct PoppingIter<'a, T: 'a>(&'a mut Vec<T>);
impl<'a, T> Iterator for PoppingIter<'a, T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.0.pop()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0.len(), Some(self.0.len()))
    }
}
impl<'a, T> ExactSizeIterator for PoppingIter<'a, T> {}

/// Applies all the specified insertions into the target vector.
///
/// The insertion iterator must be sorted in reverse order and give the proper size for its `ExactSizeIterator`.
/// Violating these constraints will never cause undefined behavior,
/// since internally we use the completely safe `BulkShifter` abstraction.
pub fn apply_bulk_insertions<T, I>(target: &mut Vec<T>, mut insertions: I)
where
    I: Iterator<Item = Insertion<T>>,
    I: ExactSizeIterator,
{
    let mut shifter = BulkShifter::new(target, insertions.len());
    /*
     * We perform insertions in reverse order to reduce moving memory,
     * and ensure that the function is panic safe.
     *
     * For example, given the vector
     * and the InsertionSet `[(0, 0), (1, 2), (1, 3) (4, 9)]`:
     *
     * Since the first (working backwards) insertion is `(4, 9)`,
     * we need to to shift all elements after our first insertion
     * to the left 4 places:
     * `[1, 4, 5, 7, undef, undef, undef, undef, 11]`.
     * The element `11` will never need to be moved again,
     * since we've already made room for all future insertions.
     *
     * Next, we perform our first insertion (4, 9) at the last `undef` element:
     * `[1, 4, 5, 7, undef, undef, undef, 9, 11]`.
     * We only have 3 insertions left to perform,
     * so all future shifts will only need to move over two.
     * Then, we handle the group of insertions `[(1, 2), [(1, 3)]`,
     * and shift all elements past index 1 to the left 3 spaces:
     * [1, undef, undef, undef, 4, 5, 7, 9, 11].
     * Then we perform our desired insertions at index 1:
     * [1, undef, 2, 3, 4, 9, 11].
     * Finally, we perform the same process for the final insertion (0, 0),
     * resulting in the desired result: [0, 1, 2, 3, 4, 9, 11].
     */
    while !shifter.is_finished() {
        let Insertion { index, element } = insertions.next().expect("Expected more insertions!");
        shifter.shift_original(index);
        shifter.push_shifted(element);
    }
    shifter.finish();
    assert_eq!(insertions.len(), 0, "Unexpected insertions");
}

/// The original location of an element (before a set of insertions are applied)
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OriginalLocation {
    /// The element was a queued insertion with the specified index
    Insertion(usize),
    /// The element was originally part of the vector
    Original(usize),
}

/// Compute the updated locations of all elements (original + inserted).
///
/// See [InsertionSet::compute_updated_locations] for details
pub fn compute_updated_locations<T, I, F>(target: &[T], mut insertions: I, mut updated: F)
where
    I: Iterator<Item = usize>,
    I: ExactSizeIterator,
    F: FnMut(OriginalLocation, usize),
{
    // This mirrors `apply_bulk_insertions` without actually shifting memory
    let mut original_len = target.len();
    let shifted_end = original_len + insertions.len();
    let mut shifted_start = shifted_end;
    let mut insertion_id = 0;
    while original_len != shifted_start {
        let insertion_index = insertions.next().expect("Expected more insertions!");
        assert!(
            insertion_index <= original_len,
            "Invalid insertion index {} > len {}",
            insertion_index,
            original_len
        );
        let moved_memory = original_len - insertion_index;
        if moved_memory > 0 {
            assert!(
                shifted_start >= moved_memory && insertion_index <= shifted_start - moved_memory
            );
            update_range(
                insertion_index..original_len,
                shifted_start - moved_memory,
                &mut updated,
            );
            shifted_start -= moved_memory;
            original_len = insertion_index;
        }
        assert!(shifted_start > original_len);
        shifted_start -= 1;
        updated(OriginalLocation::Insertion(insertion_id), shifted_start);
        insertion_id += 1;
    }
    for original_index in 0..original_len {
        updated(OriginalLocation::Original(original_index), original_index);
    }
    assert_eq!(insertions.len(), 0, "Unexpected insertions");
}
#[inline]
fn update_range<F: FnMut(OriginalLocation, usize)>(
    original: Range<usize>,
    updated_start: usize,
    func: &mut F,
) {
    let mut updated = updated_start;
    for original_index in original {
        func(OriginalLocation::Original(original_index), updated);
        updated += 1;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn basic() {
        /*
         * For example, given the vector `[1, 4, 5, 7, 11]`
         * and the InsertionSet `[(0, 0), (1, 2), (1, 3) (4, 9)]`:
         */
        let vector = vec![1, 4, 5, 7, 11];
        let insertions = [(0, 0), (1, 2), (1, 3), (4, 9)]
            .iter()
            .cloned()
            .collect::<InsertionSet<u32>>();
        assert_eq!(insertions.applied(vector), vec![0, 1, 2, 3, 4, 5, 7, 9, 11]);
    }
    #[test]
    fn updated_locations() {
        /*
         * For example, given the vector `[1, 4, 5, 7, 11]`
         * and the InsertionSet `[(0, 0), (1, 2), (1, 3) (4, 9)]`:
         */
        let vector = vec![1, 4, 5, 7, 11];
        let mut insertions = [(0, 0), (1, 2), (1, 3), (4, 9)]
            .iter()
            .cloned()
            .collect::<InsertionSet<u32>>();
        assert_eq!(
            insertions.list_updated_locations(&vector),
            vec![
                (OriginalLocation::Insertion(0), 0),
                (OriginalLocation::Original(0), 1),
                (OriginalLocation::Insertion(1), 2),
                (OriginalLocation::Insertion(2), 3),
                (OriginalLocation::Original(1), 4),
                (OriginalLocation::Original(2), 5),
                (OriginalLocation::Original(3), 6),
                (OriginalLocation::Insertion(3), 7),
                (OriginalLocation::Original(4), 8),
            ]
        );
    }
    #[test]
    fn empty_updated_locations() {
        let vector = vec![1, 4, 5, 7, 11];
        assert_eq!(
            InsertionSet::new().list_updated_locations(&vector),
            vec![
                (OriginalLocation::Original(0), 0),
                (OriginalLocation::Original(1), 1),
                (OriginalLocation::Original(2), 2),
                (OriginalLocation::Original(3), 3),
                (OriginalLocation::Original(4), 4),
            ]
        );
    }
}
