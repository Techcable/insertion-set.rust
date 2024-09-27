use std::fmt::{self, Debug, Formatter};
use std::{ptr, slice};

/// A completely safe interface for shifting a vector's elements in bulk.
///
/// This allows bulk insertions, deletions, and shifting to be done safely in-place.
/// First, we require that the additional space be .
/// We reserve enough memory for all the  insertions and operations in advanced,
/// guaranteeing that all memory `target.len() + desired_insert`,
/// and that the range (
///
/// While work is in progress, we no only consider the normal range of initialized memory `[0, target.len())`,
/// but also consider our own range of initialized memory `[shifted_start, shifted_end]`.
/// This leaves the middle range `[target.len(), shifted_start)` completely uninitialized memory,
/// and gives us room to perform our own insertions.
/// We're completely finished as soon as soon as `target.len() == shifted_start`,
/// and there's no longer uninitialized memory in the middle.
///
/// ## Example
/// 1. Assume we're given a 5-element vector of `[1, 4, 5, 7, 11]`,
///    and want to insert the value `1` at index `4`.
///   1. Since we haven't created a [`BulkShift`] object yet,
///      all the elements occupy the normal (original) range of `(0, 5)`
/// 2. First, we create a new [`BulkShift`] object with `desired_insertions = 1`,
///    which will reserve space for 1 additional element, giving the memory `[1, 4, 5, 7, 11, undef]`
///   1. Original range: `[0, 5)` has 5 defined elements.
///   2. Middle range: `[5, 6)` has 1 _undefined_ element.
///   3. Shifted (final) range: `[6, 6)` has 0 _defined_ elements.
/// 2. Move the element `11` from the original (left) side to the shifted (right) side
///     giving the memory `[1, 4, 5, 7, undef, 11]`.
///  1. Original range: `[0, 5)` has 4 defined  elements (instead of 5).
///  2. Middle range: `[5, 6)` has 1 undefined element (but changed position)
///  2. Shifted range: `(5, 6)` has 1 defined element (instead of 0).
/// 3. Insert the
pub struct BulkShifter<'a, T: 'a> {
    /// The target vector we're working with
    target: &'a mut Vec<T>,
    /// The inclusive start of the elements that have been shifted.
    /// For example, in [1, 2, undef, 3] the shifted_start is 3.
    ///
    /// This changes as we shift more and more elements,
    /// performing the desired insertions along the way.
    /// Eventually `shifted_start == target.len()`,
    /// and we will have no more space left for the inserted elements.
    /// However, we should have no more insertions left to do and should've completed our operation.
    shifted_start: usize,
    /// The exclusive end index of the elements that have been shifted
    ///
    /// This never changes, since we can only use the pre-allocated
    /// room we've already reserved.
    shifted_end: usize,
}
impl<'a, T: 'a> BulkShifter<'a, T> {
    pub fn new(target: &'a mut Vec<T>, desired_insertions: usize) -> Self {
        target.reserve(desired_insertions);
        let shifted_end = target.len() + desired_insertions;
        BulkShifter {
            target,
            shifted_end,
            shifted_start: shifted_end,
        }
    }
    /// Determines if there is remaining elements in the middle,
    /// and we're finished.
    #[inline]
    pub fn is_finished(&self) -> bool {
        self.shifted_start == self.target.len()
    }

    /// Shifts all the values after the specified original `start`
    /// from the original values over to the shifted values.
    ///
    /// Returns an [`InsufficientRoomError`] if there's not enough space to continue,
    /// since all these operations are done in place.
    #[inline]
    pub fn shift_original(&mut self, start: usize) {
        assert!(start <= self.len());
        let moved_memory = self.len() - start;
        if moved_memory == 0 {
            return;
        }
        /*
         * Since we need to allow overlapping copies,
         * we check if the `start` overlaps with the elements instead of the original `len`.
         * We need to be safe in the face of overflow,
         */
        assert!(
            self.shifted_start >= moved_memory && start <= self.shifted_start - moved_memory,
            "Insufficient room to move from {}",
            start
        );
        unsafe {
            ptr::copy(
                self.target.as_mut_ptr().add(start),
                self.target
                    .as_mut_ptr()
                    .add(self.shifted_start - moved_memory),
                moved_memory,
            );
            self.shifted_start -= moved_memory;
            self.target.set_len(start);
        }
    }
    /// Push the specified value to the start of the shifted elements
    #[inline]
    pub fn push_shifted(&mut self, value: T) {
        assert!(self.shifted_start > self.len(), "Insufficient room!");
        unsafe {
            self.shifted_start -= 1;
            ptr::write(self.target.as_mut_ptr().add(self.shifted_start), value);
        }
    }
    /// The length of the valid original elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.target.len()
    }
    #[inline]
    pub fn finish(self) -> &'a mut Vec<T> {
        assert!(self.is_finished(), "Unfinished");
        unsafe {
            self.target.set_len(self.shifted_end);
        }
        self.target
    }
    /// Slice the elements that have been shifted to the right
    #[inline]
    pub fn shifted_elements(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(
                self.target.as_ptr().add(self.shifted_start),
                self.shifted_len(),
            )
        }
    }
    /// The number of elements that have been shifted to the right
    #[inline]
    pub fn shifted_len(&self) -> usize {
        self.shifted_end - self.shifted_start
    }
}
impl<'a, T: Debug + 'a> Debug for BulkShifter<'a, T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("BulkShifter")
            .field("target", &self.target)
            .field("shifted_start", &self.shifted_start)
            .field("shifted", &self.shifted_elements())
            .finish()
    }
}
