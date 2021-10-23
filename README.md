# `pkgstrap`

An alternative to `git submodules` or `git subtree` that just works, and more!

Current State: Proof of Concept, **do not use**

## Motivation

Organizational mixed or polyrepo structures often have countless interdependencies.
This brings challenges like initial download and updating of those in the easiest case.
More often than not it is also desirable to be able to make changes to multiple projects
at once and to evaluate them before merging all of them upstream.

`pkgstrap` is meant to manage all of those dependencies that don't fit into
an existing package management solution (like NPM, Cargo, etc).
These include Git repositories, but also build / release artifacts.

## (Planned) Features

* Management of dependency versions in a Git-revisioned `pkgstrap.ron`-config
* Local overrides (ignored by Git) to use a dependency with one's own changes
* Symlinks to conveniently link from in-tree folders to out of tree repos
* Download of released artifacts from GitHub and Azure Artifacts

## Planned command-line usage (made up)

```shell
pkgstrap # initializes and or updates all dependencies (to the version specified in the config)
```

```shell
pkgstrap -c foo ../foo # initialize / update but create override for `foo` & clone it in order to work locally
```

```shell
pkgstrap -p foo ../foo # initialize / update but create override for `foo` using the specified path
```

```shell
pkgstrap -C ../ # clone & override all dependencies into ../
```

```shell
pkgstrap --pedantic --no-overrides # for continuous integration
```

## Why not `git submodules` or `git subtree`?

Both of them add additional complexity the users need to understand. The former is very error-prone,
especially when it comes to merging. And while subtrees fix some issues of submodules, they essentially
create new ones if you want to commit changes to both repos.

`pkgstrap` uses simple, easy to grasp git concepts and only adds two config files, which makes it
a lot easier in my opinion. Both read-only fetching of dependencies and working & updating sub-repos
can be done easily.
