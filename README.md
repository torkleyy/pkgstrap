# `pkgstrap`

![Status](https://img.shields.io/badge/Status-Proof%20of%20Concept-blue?style=flat-square)

**Update**: Just use [gitman](https://github.com/jacebrowning/gitman)

Turns out what I wanted to build already exists :)

## Old README

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
# initializes and or updates all dependencies (to the version specified in the config)
pkgstrap
```

```shell
# initialize / update but create override for `foo` & clone it in order to work locally
pkgstrap -c foo ../foo
```

```shell
# initialize / update but create override for `foo` using the specified path
pkgstrap -p foo ../foo
```

```shell
# override & clone all dependencies into ../
pkgstrap -C ../
```

```shell
# for continuous integration
pkgstrap --pedantic --no-overrides
```

## Why not `git submodules` or `git subtree`?

Both of them add additional complexity the users need to understand. The former is very error-prone,
especially when it comes to merging. And while subtrees fix some issues of submodules, they essentially
create new ones if you want to commit changes to both repos.

`pkgstrap` uses simple, easy to grasp git concepts and only adds two config files, which makes it
a lot easier in my opinion. Both read-only fetching of dependencies and working & updating sub-repos
can be done easily.
