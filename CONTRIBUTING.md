# Welcome to RAstra Contributing Guide

Thank you for your interest in contributing to **RAstra** 🎨  
Every contribution — code, documentation, bug reports, or ideas — is highly appreciated.

This guide explains how to contribute effectively, from reporting issues to submitting pull requests and getting them merged.

---

## New contributors

If you are new to the project, start by reading the [README](README.md) to understand RAstra's goals and architecture.

Helpful resources:
- [How to contribute to open source on GitHub](https://docs.github.com/en/get-started/exploring-projects-on-github/finding-ways-to-contribute-to-open-source-on-github)
- [Setting up Git](https://docs.github.com/en/get-started/getting-started-with-git/set-up-git)
- [GitHub Flow](https://docs.github.com/en/get-started/using-github/github-flow)
- [Working with Pull Requests](https://docs.github.com/en/github/collaborating-with-pull-requests)

---

## Getting started

### Issues

#### Reporting bugs or suggesting features

Before opening a new issue, please check the existing issues to avoid duplicates:  
👉 https://github.com/bedrock-crustaceans/RAstra/issues

If no related issue exists, open a new one using the appropriate issue template and provide as much detail as possible:
- Expected behavior
- Actual behavior
- Steps to reproduce
- Logs, screenshots, or stack traces (if applicable)

#### Working on an issue

Browse the issue list and look for something you’d like to work on.  
Issues are **not assigned** by default so feel free to start working on any open issue and submit a PR.

Using labels is recommended to find suitable tasks.

---

## Making changes

### Setup

1. Fork the repository
2. Clone your fork locally
3. Install Rust (if not)

### Development workflow

1. Create a new branch for your changes
   ```bash
   git checkout -b feature/my-change
   ````

2. Implement your changes

    * Follow the existing code style
    * Keep changes focused and minimal
    * Avoid unrelated refactors in the same PR

3. Run the server locally to verify your changes:

---

## Pull Requests

When your changes are ready:

1. Push your branch to your fork
2. Open a Pull Request against the main repository
3. Fill out the **Pull Request template** completely

### PR guidelines

* Clearly describe **what** and **why** you changed
* Link related issues using keywords (e.g. `Fixes #123`)
* Keep PRs small and focused
* Enable **“Allow maintainer edits”**
* Ensure all checks pass before requesting review

Reviewers may:

* Ask for clarifications
* Request changes
* Suggest improvements directly in the code

Please address feedback and mark conversations as resolved once fixed.

If you encounter merge conflicts, refer to this guide:
[https://github.com/skills/resolve-merge-conflicts](https://github.com/skills/resolve-merge-conflicts)

---

## After merge

🎉 Congratulations, and thank you for contributing to **RAstra**!

Once merged:

* Your contribution becomes part of the project
* Your GitHub profile will reflect your contribution
* You are officially part of the RAstra community

We’re glad to have you here. 💙