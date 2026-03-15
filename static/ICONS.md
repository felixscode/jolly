# Jolly Icons

Hand-drawn SVG sprites for the Jolly bird character. Created in Inkscape.

## States

| File | State |
|------|-------|
| `jolly_normal.svg` | Idle — default resting pose |
| `jolly_talk.svg` | Talking — beak open |
| `jolly_thinking.svg` | Thinking — contemplative pose |
| `jolly_fly1.svg` | Flight frame 1 |
| `jolly_fly2.svg` | Flight frame 2 |
| `jolly_dead.svg` | Error / crash state |
| `jolly_heading.svg` | Hero heading variant |
| `jolly_bilnzel.svg` | Blink / squint |

## Props

| File | Description |
|------|-------------|
| `jolly_tree.svg` | Branch / perch environment |
| `jolly_thinking_bubble.svg` | Thought bubble overlay |

## Usage

Import directly as Svelte components via `vite-plugin-svgr`, or reference from `/static`:

```svelte
<img src="/jolly_normal.svg" alt="Jolly" />
```

## Colors

All icons use the brand palette — dark navy `#241e4e`, red `#960200`, yellow `#ffd046`, tan `#aea897`.
