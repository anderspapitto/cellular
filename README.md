cellular provides a unified least-common-denominator interface between
UnsafeCell and RefCell, to enable toggling between them with a generic
parameter. The primary use case is to allow running tests with RefCell in
place of UnsafeCell for ease of debugging (safe panics rather than segfaults
and memory corruption).

Usage example:
```
  use cellular::{Cellular, CellularStrict, MkCell, MkCellSafe, UnsafeOptIn};

  // Minimal one-time boilerplate (see Safety below)
  struct IPromiseToBehave {}                      // client defines a dummy struct
  unsafe impl UnsafeOptIn for IPromiseToBehave {} // client impls an unsafe marker trait
  type MkCellFast = cellular::MkCellFast<IPromiseToBehave>;

  fn example_generic<MkC: MkCell>() {
    let x = MkC::C::from(0);
    assert_eq!(*x.ro(), 0);  // ro() is like borrow()
    *x.rw() = 1;             // rw() is like borrow_mut()
    assert_eq!(*x.ro(), 1);
    let x = x.into_inner();
    assert_eq!(x, 1);
  }

  fn go() {
    example_generic::<MkCellSafe>(); // uses RefCell
    example_generic::<MkCellFast>(); // uses UnsafeCell
  }
```

There is also an encoding of HKTs via MkCell, to make it ergonomic to thread
through just one toggleable generic parameter.

Safety:
  This library is intended for those who have *already* decided to use
  UnsafeCell (and therefore unsafe)!

  This library provides both a by-the-book CellularStrict interface as well
  as a more ergonomic Cellular interface with a novel opt-in mechanism.

  If you use CellularStrict, ro_strict() and rw_strict() are unsafe, because
  the backing operations on UnsafeCell are (of course) unsafe. You are
  responsible for using these correctly, exactly as if you were using
  UnsafeCell directly (but you still gain the ability to easily swap to the
  RefCell implementation).

  If you use Cellular, you are (of course) also still responsible in the
  same way. However, ro() and rw() are *not* marked as unsafe, and instead
  you acknowledge your responsibility once up front instead of at each usage
  site by implementing the unsafe marker trait UnsafeOptIn.
