# Clean Code & UI Standards — Agent Reference

---

## 1. Naming
- Use descriptive, intention-revealing names. A reader should understand purpose without needing comments.
- Avoid abbreviations, single-letter variables (except simple loop indices), and generic names like `data`, `temp`, `result`.
- Booleans should read as yes/no questions: `isActive`, `hasPermission`, `shouldRetry`.
- Functions should be verbs or verb phrases: `calculateTotal`, `fetchUser`, `validateInput`.

## 2. Functions
- Each function should do **one thing**. If you can describe it with "and", split it.
- Keep functions short — aim for under 20 lines. If it's longer, extract helpers.
- Limit parameters to 3 or fewer. If more are needed, group them into an object/struct.
- Avoid flag/boolean parameters — they signal the function does two things.
- Return early to avoid deep nesting. Prefer guard clauses over nested if/else.

## 3. Structure & Organisation
- Follow the **Single Responsibility Principle** — each file, class, or module has one reason to change.
- Group code by feature or domain, not by type (e.g., not all controllers in one folder).
- Keep related code close together. A reader shouldn't have to jump between files to understand a flow.
- Limit file length. If a file exceeds ~200–300 lines, consider splitting it.

## 4. Comments & Readability
- Code should be self-documenting. If you need a comment to explain *what*, rename or restructure instead.
- Comments should explain **why**, not what. Reserve them for non-obvious business logic or trade-offs.
- Delete commented-out code — version control exists for a reason.
- Avoid inline magic numbers or strings. Extract them into named constants.

## 5. Error Handling
- Handle errors explicitly. Never silently swallow exceptions or errors.
- Fail fast — validate inputs at the boundary and return/throw early.
- Use meaningful error messages that include context (what failed, why, and what input caused it).
- Separate error handling logic from business logic.

## 6. DRY & Abstraction
- **Don't Repeat Yourself** — if logic appears in two or more places, extract it.
- But don't over-abstract prematurely. Wait for the **Rule of Three**: duplicate once is okay, duplicate twice means extract.
- Prefer composition over inheritance.
- Keep abstractions at a consistent level within a function — don't mix high-level orchestration with low-level detail.

## 7. Testing & Maintainability
- Write code that is easy to test. If it's hard to test, it's probably too coupled.
- Pure functions (same input → same output, no side effects) are preferred where possible.
- Minimise mutable state and side effects. Isolate side effects to the edges of the system.
- Avoid global state.

## 8. General Principles
- **YAGNI** — You Aren't Gonna Need It. Don't build for hypothetical future requirements.
- **Boy Scout Rule** — Leave the code cleaner than you found it.
- **Least Surprise** — Code should behave the way a reader would expect from its name and signature.
- **Prefer explicit over clever.** Readable, boring code beats clever, compact code every time.

---

## 9. Component Design
- Each component should have a **single responsibility**. If it manages layout AND business logic, split them.
- Keep components small and composable. Prefer many small components over one large monolith.
- Separate **presentational components** (how things look) from **container/logic components** (how things work).
- Components should be self-contained — avoid reaching up into parent state or deeply into child internals.
- Design components for reuse by default: accept props/inputs, avoid hardcoded values.

## 10. State Management
- Keep state as **local as possible**. Only lift state up when multiple components genuinely need it.
- Avoid duplicating state — derive computed values rather than storing them separately.
- Distinguish between **UI state** (modals open, tabs selected) and **server/data state** (fetched records). Treat them differently.
- Minimise global/shared state. The more global state you have, the harder the app is to reason about.
- State should flow in **one direction** — parent to child. Avoid bidirectional data flows.

## 11. Layout & Styling
- Use **semantic HTML elements** (`<nav>`, `<main>`, `<section>`, `<button>`) instead of generic `<div>` and `<span>` for everything.
- Avoid inline styles for anything beyond truly dynamic values (e.g., calculated positions).
- Use a consistent spacing and sizing system (e.g., 4px/8px grid). Don't use arbitrary pixel values.
- Keep styles **co-located** with components. A component's styles should live near the component, not in a distant global stylesheet.
- Avoid deeply nested selectors. Flat, scoped styles are easier to maintain and override.
- Use design tokens or CSS variables for colours, fonts, spacing, and breakpoints — never hardcode values.

## 12. Responsiveness & Accessibility
- Design **mobile-first**, then layer on larger breakpoints.
- Use relative units (`rem`, `em`, `%`, `vh/vw`) over fixed `px` where appropriate.
- Every interactive element must be **keyboard accessible** — focusable, operable via Enter/Space, with visible focus indicators.
- All images need meaningful `alt` text (or `alt=""` if purely decorative).
- Use proper **ARIA roles and labels** when semantic HTML alone isn't sufficient. Don't add ARIA to elements that already have native semantics.
- Maintain a **colour contrast ratio** of at least 4.5:1 for body text and 3:1 for large text (WCAG AA).
- Never rely on colour alone to convey meaning — use icons, text, or patterns as well.

## 13. User Interaction & Feedback
- Every user action should have **visible feedback** — loading spinners, disabled states, success/error messages.
- Show loading states for any async operation. Never leave the user staring at a frozen screen.
- Use **optimistic updates** where safe, but always handle the failure case gracefully.
- Disable or indicate busy state on buttons/forms during submission to prevent double-actions.
- Error messages should be **specific and actionable** — "Email format is invalid" not "Error occurred".
- Place error messages **near the relevant input**, not only in a generic toast/banner.

## 14. Performance
- Avoid unnecessary re-renders. Only re-render components when their actual inputs change.
- **Lazy-load** heavy components, routes, and images that aren't immediately visible.
- Debounce or throttle expensive handlers (search inputs, scroll events, resize listeners).
- Minimise DOM depth — deeply nested elements hurt rendering performance and accessibility.
- Optimise images: use appropriate formats (WebP/AVIF), serve responsive sizes, and always set explicit dimensions to prevent layout shift.

## 15. Forms
- Validate inputs on **blur and on submit**, not only on submit.
- Preserve user input on validation failure — never clear the form.
- Mark required fields clearly. Use `required` attributes and visual indicators.
- Group related fields logically and use `<fieldset>` / `<legend>` where appropriate.
- Support autofill — use correct `name`, `type`, and `autocomplete` attributes.

## 16. Navigation & Routing
- Use real links (`<a>` / router links) for navigation, not click handlers on divs or buttons.
- URLs should be **readable, bookmarkable, and shareable**. Avoid opaque IDs when possible.
- Preserve and restore scroll position on back navigation.
- Handle 404 / not-found routes gracefully with a helpful fallback page.

## 17. UI General Principles
- **Progressive enhancement** — core functionality should work first, then layer on enhancements.
- **Don't block the main thread.** Move heavy computation off the UI thread.
- **Consistency over novelty** — follow established UI patterns. Users shouldn't have to learn your custom interaction model.
- **Test from the user's perspective** — test what the user sees and does (clicks, text on screen), not implementation details.

---

*When in doubt: optimise for readability. Code is read far more often than it is written. Build for the user who is on a slow connection, using a keyboard, and seeing your UI for the first time.*
