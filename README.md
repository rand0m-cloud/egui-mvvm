# `egui-mvvm`

A minimal MVVM-style framework for building reactive, async-capable UIs using [`egui`](https://github.com/emilk/egui).

---

## 🧠 Key Concepts

This project draws inspiration from my time with Jetpack Compose, which remains my favorite approach to UI architecture.
UI development can be frustrating — especially when the tools fight your intentions. This framework tries to make things
feel cooperative again.

`egui-mvvm` brings Model-View-ViewModel (MVVM) to the `egui` ecosystem, with a focus on:

- Clean separation of concerns
- Async-aware ViewModel logic
- Latching state for predictable UI and task behavior

---

## 🧱 ViewModel: A Living State

In this framework, the **ViewModel** is the central abstraction.

* Think of the ViewModel as the **living state** of your UI.
* It evolves over time, reacts to business events, and performs async operations.
* Your UI — the **View** — operates on a handle to the ViewModel.
* The **Model** is a snapshot of the current state derived from the ViewModel.

### Why ViewModels?

* ViewModels are persistent for as long as the user “wants to see” them.
* They provide a clear space for managing both **reactive state** and **async logic**.
* Each ViewModel owns a `TaskPool`, letting you:

    * Launch async tasks to produce future state.
    * Automatically cancel tasks when the ViewModel is discarded (no stale logic or dangling updates).

---

## 🧬 What Are ViewModels Made Of?

ViewModels are built from **stateful primitives** that act like **streams of value changes** — powering a reactive UI
model where views reflect evolving data.

### State Primitives

* **`RefState<T>`**
  Designed for **expensive-to-clone** types.

    * Uses reference-based mutation.
    * Clones only when necessary.
    * Great for documents, trees, or large structs.

* **`ValState<T>`**
  Designed for **cheap-to-clone** types.

    * Updates by cloning the value.
    * Great for booleans, numbers, small structs.

### Async Task Execution

Each ViewModel includes a built-in `TaskPool`:

* Launch futures using `.spawn()` with a handle to send state updates.
* Cancel running tasks automatically when the ViewModel is dropped.
* Enables clean and lifecycle-safe async logic directly within the ViewModel.

---

## ✨ Motivating Example: Async State in Action

Here's a simplified example of a `CommentViewModel` that tracks a simulated upload:

```rust
view_model! {
    #[view]
    pub struct CommentView {
        #[viewmodel]
        pub view_model: &mut CommentViewModel,
    }

    #[viewmodel(default)]
    pub struct CommentViewModel {
        pub status: ValState<Option<Status>> = None,
        pub error: ValState<Option<Error>> = None,
        pub text: RefState<String> = "".to_string(),
    }
}

impl CommentViewModel {
    pub fn simulate_upload(&self) {
        // Guard: already uploading
        if matches!(self.status.value(), Some(Status::Uploading)) {
            return;
        }

        // Kick off upload
        self.status.send_value(Some(Status::Uploading));

        // `this` is a collection of handles to send state updates.
        self.spawn(|this| async move {
            // Simulate delay
            tokio::time::sleep(Duration::from_secs(2)).await;

            // Update state
            this.status.send_value(Some(Status::Success));
            this.text.send_update(|content| {
                *content = format!("Uploaded: {}", content);
            });
        });
    }
}
```

This example showcases the power of the `view_model!` macro while illustrating the core design philosophy: the UI reads
from the ViewModel’s state and sends business events back to it, allowing the ViewModel to process them and produce
updated state over time.

## 🪝 Hooks: Handy but Not Primary

While `egui-mvvm` is primarily designed around explicit ViewModels and state primitives, a small set of hooks are
provided for convenience.

Hooks aren’t the recommended way to structure state but can be very handy in specific cases or simple
components.

Currently implemented hooks:

- `use_val_state` — for simple value state
- `use_ref_state` — for reference-backed state
- `use_effect` — to run side effects
- `use_debounce` — to debounce rapid state changes or events

These hooks complement the core API but are auxiliary tools rather than the main pattern.

## ❌ Known Limitations & Gripes

While `egui-mvvm` is designed around state latching and scoped async logic, these ideas are still evolving and not fully
baked in this repository:

- **State Latching Is Partial**

The current system does not fully realize the ideal of latched snapshots like Jetpack Compose’s Snapshot system, where
state writes are batched and only become visible on the next UI pass.
This means in some scenarios, tasks or UI updates may see intermediate or inconsistent state changes, weakening the
guarantees we want around predictability.

- **RefState API Needs Work**

The RefState primitive — aimed at supporting in-place mutation of complex data — currently has a clunky API for sending
updates.
It could be improved to better fit ergonomic usage patterns and fully deliver on its goal of efficiently mutating large
or shared state.

- **RefState/ValState over Strings need a TextBuffer implementation**

Editing text is a bit clunky because `egui` wants `&mut` access and `egui-mvvm` needs to know when the text has changed.
The examples currently use the untracked `&mut` access and `mark_changed` when `egui` tells us its changed. 
