use std::collections::BTreeMap;

use anyhow::Result;
use anyhow::anyhow;
use indicatif::ProgressBar;

use crate::repository::Repository;
use crate::package::Package;
use crate::package::version::VersionParser;
use crate::util::executor::Executor;

pub struct Tree {
    root: BTreeMap<Package, Tree>,
}

impl Tree {

    pub fn new() -> Self {
        Tree { root: BTreeMap::new() }
    }

    pub fn add_package(&mut self, p: Package, repo: &Repository, executor: &dyn Executor, versionparser: &dyn VersionParser, progress: &ProgressBar) -> Result<()> {
        macro_rules! mk_add_package_tree {
            ($this:ident, $pack:ident, $repo:ident, $root:ident, $executor:ident, $versionparser:ident, $progress:ident) => {{
                let mut subtree = Tree::new();
                ($pack).get_all_dependencies($executor, $versionparser)?
                    .into_iter()
                    .map(|(name, constr)| {
                        let pack = ($repo).find_with_version_constraint(&name, &constr);

                        if pack.iter().any(|p| ($root).has_package(p)) {
                            // package already exists in tree, which is unfortunate
                            // TODO: Handle gracefully
                            //
                            return Err(anyhow!("Duplicate version of some package in {:?} found", pack))
                        }

                        pack.into_iter()
                            .map(|p| {
                                ($progress).tick();
                                add_package_tree(&mut subtree, p.clone(), ($repo), ($root), ($executor), ($versionparser), ($progress))
                            })
                            .collect()
                    })
                    .collect::<Result<Vec<()>>>()?;

                ($this).root.insert(($pack), subtree);
                Ok(())
            }}
        };

        fn add_package_tree(this: &mut Tree, p: Package, repo: &Repository, root: &mut Tree, executor: &dyn Executor, versionparser: &dyn VersionParser, progress: &ProgressBar) -> Result<()> {
            mk_add_package_tree!(this, p, repo, root, executor, versionparser, progress)
        }

        mk_add_package_tree!(self, p, repo, self, executor, versionparser, progress)
    }

    pub fn has_package(&self, p: &Package) -> bool {
        let name_eq = |k: &Package| k.name() == p.name();
        self.root.keys().any(name_eq) || self.root.values().any(|t| t.has_package(p))
    }

    /// Find how deep the package is in the tree
    ///
    /// # Returns
    ///
    /// * None if the package is not in the tree
    /// * Some(usize) with the depth of the package in the tree, where the package at the root of
    /// the tree is treated as 0 (zero)
    ///
    /// # Note
    ///
    /// If the package is multiple times in the tree, only the first one will be found
    // TODO: Remove allow(unused)
    #[allow(unused)]
    pub fn package_depth(&self, p: &Package) -> Option<usize> {
        self.package_depth_where(|k| k == p)
    }

    /// Same as `package_depth()` but with custom compare functionfunction
    // TODO: Remove allow(unused)
    #[allow(unused)]
    pub fn package_depth_where<F>(&self, cmp: F) -> Option<usize>
        where F: Fn(&Package) -> bool
    {
        fn find_package_depth<F>(tree: &Tree, current: usize, cmp: &F) -> Option<usize>
            where F: Fn(&Package) -> bool
        {
            if tree.root.keys().any(|k| cmp(k)) {
                return Some(current)
            } else {
                tree.root
                    .values()
                    .filter_map(|subtree| find_package_depth(subtree, current + 1, cmp))
                    .next()
            }
        }

        find_package_depth(self, 0, &cmp)
    }

}
