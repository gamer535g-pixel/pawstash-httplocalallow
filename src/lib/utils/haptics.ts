export function haptic(ms = 35) {
  try {
    navigator.vibrate?.(ms);
  } catch {
    // Some WebViews throw instead of returning false.
  }
}
