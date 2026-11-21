# AGENTS.md

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/):

```text
<type>(<optional scope>): <subject>

<optional body>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`,
`ci`, `chore`, `revert`.

- Use the imperative mood and lowercase subject, no trailing period.
- Keep the subject under ~72 characters.
- Mark breaking changes with `!` after the type/scope and explain them in a
  `BREAKING CHANGE:` footer.
- Reference related OpenSpec changes or issues in the body when useful.

Examples:

```text
feat(scan): group media by YYYY/MM folder
fix(thumbs): retry failed posters on the next run
docs(readme): document nginx deployment
```
