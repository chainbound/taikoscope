# Dashboard Agent Guidelines

## Build & Serve
- Install dependencies with `npm install`.
- Start the dev server with `npm run dev`.
- Build for production with `npm run build`.
- Run tests with `npm run test` or `just test-dashboard`.
- Run type checks with `npm run check` or `just check-dashboard` whenever you modify dashboard code and ensure this passes before opening a PR.
- Always run `just ci` after any changes.

## Code Style
- Use TypeScript and keep components typed.
- Format using `prettier`.
- Avoid lines with trailing whitespace (spaces or tabs)

## Git
- Use Conventional Commits for commits.
