//! cellular provides a unified least-common-denominator interface between
//! UnsafeCell and RefCell, to enable toggling between them with a generic
//! parameter. The primary use case is to allow running tests with RefCell in
//! place of UnsafeCell for ease of debugging (safe panics rather than segfaults
//! and memory corruption).
//!
//! Usage example:
//! ```
//!   use cellular::{Cellular, CellularStrict, MkCell, MkCellSafe, UnsafeOptIn};
//!
//!   // Minimal one-time boilerplate (see Safety below)
//!   struct IPromiseToBehave {}                      // client defines a dummy struct
//!   unsafe impl UnsafeOptIn for IPromiseToBehave {} // client impls an unsafe marker trait
//!   type MkCellFast = cellular::MkCellFast<IPromiseToBehave>;
//!
//!   fn example_generic<MkC: MkCell>() {
//!     let x = MkC::C::from(0);
//!     assert_eq!(*x.ro(), 0);  // ro() is like borrow()
//!     *x.rw() = 1;             // rw() is like borrow_mut()
//!     assert_eq!(*x.ro(), 1);
//!     let x = x.into_inner();
//!     assert_eq!(x, 1);
//!   }
//!
//!   fn go() {
//!     example_generic::<MkCellSafe>(); // uses RefCell
//!     example_generic::<MkCellFast>(); // uses UnsafeCell
//!   }
//! ```
//!
//! There is also an encoding of HKTs via MkCell, to make it ergonomic to thread
//! through just one toggleable generic parameter.
//!
//! Safety:
//!   This library is intended for those who have *already* decided to use
//!   UnsafeCell (and therefore unsafe)!
//!
//!   This library provides both a by-the-book CellularStrict interface as well
//!   as a more ergonomic Cellular interface with a novel opt-in mechanism.
//!
//!   If you use CellularStrict, ro_strict() and rw_strict() are unsafe, because
//!   the backing operations on UnsafeCell are (of course) unsafe. You are
//!   responsible for using these correctly, exactly as if you were using
//!   UnsafeCell directly (but you still gain the ability to easily swap to the
//!   RefCell implementation).
//!
//!   If you use Cellular, you are (of course) also still responsible in the
//!   same way. However, ro() and rw() are *not* marked as unsafe, and instead
//!   you acknowledge your responsibility once up front instead of at each usage
//!   site by implementing the unsafe marker trait UnsafeOptIn.

#![feature(arbitrary_self_types)]
use std::cell::{RefCell, UnsafeCell};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Receiver};

pub unsafe trait UnsafeOptIn {}

pub trait CellularStrict: From<Self::T> + Receiver<Target = Self::T> {
  type T;
  type Ref<'a>: Deref<Target = Self::T>
  where Self: 'a;
  type RefMut<'a>: DerefMut<Target = Self::T>
  where Self: 'a;
  unsafe fn ro_strict(&self) -> Self::Ref<'_>;
  unsafe fn rw_strict(&self) -> Self::RefMut<'_>;
  fn into_inner(self) -> Self::T;
}

pub trait Cellular: CellularStrict {
  fn ro(&self) -> Self::Ref<'_> { unsafe { self.ro_strict() } }
  fn rw(&self) -> Self::RefMut<'_> { unsafe { self.rw_strict() } }
}

// Rust doesn't support HKTs, so we encode it as a trait with a GAT.
pub trait MkCellStrict {
  type C<T>: CellularStrict<T = T>;
}

// We would rather have
//   pub trait MkCell: MkCellStrict where for<T> Self::C<T>: Cellular<T=T> {}
// but Rust can't really handle this.
pub trait MkCell {
  type C<T>: Cellular<T = T>;
}

#[derive(Debug, Default)]
pub struct CellSafe<T>(RefCell<T>);
pub struct MkCellSafe;

#[derive(Debug, Default)]
pub struct CellFast<Token, T>(UnsafeCell<T>, PhantomData<Token>);
pub struct MkCellFast<Token>(PhantomData<Token>);

mod cell_safe {
  use std::cell::{Ref, RefMut};

  use super::*;

  impl<T> From<T> for CellSafe<T> {
    fn from(value: T) -> Self { Self(value.into()) }
  }

  impl<T> Receiver for CellSafe<T> {
    type Target = T;
  }

  impl<T> CellularStrict for CellSafe<T> {
    type T = T;
    type Ref<'a>
      = Ref<'a, T>
    where Self: 'a;
    type RefMut<'a>
      = RefMut<'a, T>
    where Self: 'a;

    unsafe fn ro_strict(&self) -> Self::Ref<'_> { self.0.borrow() }
    unsafe fn rw_strict(&self) -> Self::RefMut<'_> { self.0.borrow_mut() }
    fn into_inner(self) -> Self::T { self.0.into_inner() }
  }

  impl<T> Cellular for CellSafe<T> {}

  impl MkCellStrict for MkCellSafe {
    type C<T> = CellSafe<T>;
  }
  impl MkCell for MkCellSafe {
    type C<T> = CellSafe<T>;
  }
}

mod cell_fast {
  use super::*;

  impl<Token, T> From<T> for CellFast<Token, T> {
    fn from(value: T) -> Self { Self(value.into(), PhantomData) }
  }

  impl<Token, T> Receiver for CellFast<Token, T> {
    type Target = T;
  }

  impl<Token, T> CellularStrict for CellFast<Token, T> {
    type T = T;
    type Ref<'a>
      = &'a T
    where Self: 'a;
    type RefMut<'a>
      = &'a mut T
    where Self: 'a;

    unsafe fn ro_strict(&self) -> Self::Ref<'_> { unsafe { &*self.0.get() } }
    unsafe fn rw_strict(&self) -> Self::RefMut<'_> { unsafe { &mut *self.0.get() } }
    fn into_inner(self) -> Self::T { self.0.into_inner() }
  }

  impl<Token: UnsafeOptIn, T> Cellular for CellFast<Token, T> {}

  impl<Token> MkCellStrict for MkCellFast<Token> {
    type C<T> = CellFast<Token, T>;
  }

  impl<Token: UnsafeOptIn> MkCell for MkCellFast<Token> {
    type C<T> = CellFast<Token, T>;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  struct IPromiseToBehave {}
  unsafe impl UnsafeOptIn for IPromiseToBehave {}
  type MkCellFast = super::MkCellFast<IPromiseToBehave>;

  fn test_generic<MkC: MkCell>() {
    let x = MkC::C::from(0);
    assert_eq!(*x.ro(), 0);
    *x.rw() = 1;
    assert_eq!(*x.ro(), 1);
    let x = x.into_inner();
    assert_eq!(x, 1);
  }

  #[test]
  fn test_cell_safe() { test_generic::<MkCellSafe>(); }

  #[test]
  fn test_cell_fast() { test_generic::<MkCellFast>(); }

  #[test]
  #[should_panic(expected = "RefCell already borrowed")]
  fn test_conflict_ro_rw() {
    let x = CellSafe::from(0);
    let _ro = x.ro();
    let _rw = x.rw();
  }

  #[test]
  #[should_panic(expected = "RefCell already borrowed")]
  fn test_conflict_rw_rw() {
    let x = CellSafe::from(0);
    let _rw1 = x.rw();
    let _rw2 = x.rw();
  }

  #[test]
  #[should_panic(expected = "RefCell already mutably borrowed")]
  fn test_conflict_rw_ro() {
    let x = CellSafe::from(0);
    let _rw = x.rw();
    let _ro = x.ro();
  }
}
