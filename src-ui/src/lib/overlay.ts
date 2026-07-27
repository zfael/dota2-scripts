/**
 * The overlay runs the same bundle as the main app, told apart by a query
 * parameter on the URL the overlay window is created with.
 *
 * A query parameter rather than a route because the app uses BrowserRouter,
 * which cannot resolve a path against a `file://`-style static load, and rather
 * than a second Vite entry point because the overlay shares every store and
 * component with the main window.
 */
export function isOverlayWindow(search: string = window.location.search): boolean {
  return new URLSearchParams(search).has("overlay");
}

/**
 * Strip the main window's chrome from `<body>`.
 *
 * The global stylesheet gives body an opaque background and a 900x650 minimum,
 * both of which break a small transparent overlay — an opaque body would simply
 * hide the minimap it is drawn over. Applied synchronously before first paint.
 */
export function applyOverlayBodyStyles(element: HTMLElement = document.body): void {
  element.classList.add("overlay-mode");
}
