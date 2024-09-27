//! Utilities for sorting.

use std::cmp::Ordering;

/// Performs an [insertion sort](https://en.wikipedia.org/wiki/Insertion_sort)
/// on the specified slice,
/// using the specified comparison function.
///
/// Unfortunately, this algorithm has quadratic worst-case complexity,
/// and is much slower then than quicksort and mergesort for large inputs.
/// Its should only be used if the input is small or already mostly sorted,
/// as described on the wikipedia page.
pub fn insertion_sort_by<T, F>(target: &mut [T], mut compare: F)
where
    F: FnMut(&T, &T) -> Ordering,
{
    for i in 1..target.len() {
        let mut j = i;
        while j > 0 && compare(&target[j - 1], &target[j]) == Ordering::Greater {
            target.swap(j, j - 1);
            j -= 1;
        }
    }
}

/// Performs an insertion sort on the specified slice,
/// comparing values using the specified function.
///
/// See [`insertion_sort_by`] for algorithm details.
#[inline]
pub fn insertion_sort_by_key<T, B, F>(target: &mut [T], mut func: F)
where
    B: Ord,
    F: FnMut(&T) -> B,
{
    insertion_sort_by(target, |first, second| func(first).cmp(&func(second)))
}
