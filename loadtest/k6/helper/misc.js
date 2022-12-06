export function random_string() {
    return Math.random().toString(36).slice(-5)
}