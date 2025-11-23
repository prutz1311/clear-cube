use crate::block::*;
use bevy::math::IVec3;
use rand::{Rng, rngs::ThreadRng};
use rand::prelude::*;

pub enum Tree {
    Leaf(GBlock),
    Node(Box<Tree>, Box<Tree>),
}

fn flatten_tree_rec(t: &Tree, acc: &mut Vec<GBlock>) -> () {
    match t {
        Tree::Leaf(x) => {
            acc.push(x.clone())
        },
        Tree::Node(l, r) => {
            flatten_tree_rec(l, acc);
            flatten_tree_rec(r, acc);
        },
    }
}

pub fn flatten_tree(t: &Tree) -> Vec<GBlock> {
    let mut acc = Vec::new();
    flatten_tree_rec(t, &mut acc);
    acc
}

pub struct Seed {
    x: (i32, i32),
    y: (i32, i32),
    z: (i32, i32)
}

impl Seed {
    pub fn split(self: &Self, axis: &Axis, mid: i32) -> (Self, Self) {
        match axis {
            Axis::X => (Self { x: (self.x.0, mid), ..(*self) }, Self { x: (mid, self.x.1), ..(*self) }),
            Axis::Y => (Self { y: (self.y.0, mid), ..(*self) }, Self { y: (mid, self.y.1), ..(*self) }),
            Axis::Z => (Self { z: (self.z.0, mid), ..(*self) }, Self { z: (mid, self.z.1), ..(*self) }),
        }
    }

    pub fn to_min_max(self: &Self) -> (IVec3, IVec3) {
        let Seed { x: (xl, xu), y: (yl, yu), z: (zl, zu) } = *self;
        (IVec3::new(xl, yl, zl), IVec3::new(xu, yu, zu))
    }

    pub fn get_field(self: &Self, axis: &Axis) -> (i32, i32) {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }
}

#[derive(PartialEq)]
pub enum Width {
    One,
    Two,
    More,
}

pub fn width(x: i32) -> Width {
    match x {
        1 => Width::One,
        2 => Width::Two,
        n if n >= 2 => Width::More,
        _ => panic!("wrong width"),
    }
}

pub fn random_direction(rng: &mut ThreadRng) -> Direction {
    let axis = match rng.random_range(0..3) {
        0 => Axis::X,
        1 => Axis::Y,
        2 => Axis::Z,
        _ => panic!("random_direction: wrong axis index"),
    };
    let positive = rng.random_bool(0.5);
    Direction { axis, positive }
}

#[derive(Debug, Clone)]
pub struct GBlock {
    pub direction: Option<Direction>,
    pub min: IVec3,
    pub max: IVec3,
}

impl GBlock {
    pub fn new(direction: Option<Direction>, min: IVec3, max: IVec3) -> Self {
        Self { direction, min, max }
    }
}

pub fn gblock_to_block(gb: &GBlock) -> Option<Block> {
    let &GBlock { direction: ref odir, min, max } = gb;
    odir.clone().map(|direction| Block { direction, min, max })
}

pub fn gblocks_to_blocks(gb: &[GBlock]) -> Vec<Block> {
    gb.iter().filter_map(gblock_to_block).collect()
}

// TODO: branches
pub fn gen_tree(rng: &mut ThreadRng, seed: Seed) -> Tree {
    let Seed { x: (xmin, xmax), y: (ymin, ymax), z: (zmin, zmax) } = seed;
    let xwidth = xmax - xmin;
    let ywidth = ymax - ymin;
    let zwidth = zmax - zmin;
    let widths: Vec<Width> = [xwidth, ywidth, zwidth]
        .iter().map(|x| width(*x)).collect();
    let ones: usize = widths.iter().filter(|w: &&Width| **w == Width::One).count();
    let twos: usize = widths.iter().filter(|w: &&Width| **w == Width::Two).count();
    let (min, max) = seed.to_min_max();
    match (ones, twos) {
        (3, 0) => {
            let filled: bool = rng.random_bool(0.5);
            if filled {
                let dir = random_direction(rng);
                Tree::Leaf(GBlock::new(Some(dir), min, max))
            }
            else {
                Tree::Leaf(GBlock::new(None, min, max))
            }
        },
        (2, 1) => {
            let axis = widths.iter()
                .zip(Axis::ALL.iter()).filter(|(w, _)| **w == Width::Two)
                .next().unwrap().1;
            let split = rng.random_bool(0.5);
            if split {
                let low = seed.get_field(axis).0;
                let mid = low + 1;
                let (low_subseed, high_subseed) = seed.split(axis, mid);
                Tree::Node(
                    Box::new(gen_tree(rng, low_subseed)),
                    Box::new(gen_tree(rng, high_subseed))
                )
            }
            else {
                let filled: bool = rng.random_bool(0.5);
                if filled {
                    let dir = random_direction(rng);
                    Tree::Leaf(GBlock::new(Some(dir), min, max))
                }
                else {
                    Tree::Leaf(GBlock::new(None, min, max))
                }
            }
        }
        (2, _) => {
            let axis = widths.iter()
                .zip(Axis::ALL.iter()).filter(|(w, _)| **w != Width::One)
                .next().unwrap().1;
            let (low, high) = seed.get_field(axis);
            let mid = rng.random_range(low + 1 ..= high - 1);
            let (low_subseed, high_subseed) = seed.split(axis, mid);
            Tree::Node(
                Box::new(gen_tree(rng, low_subseed)),
                Box::new(gen_tree(rng, high_subseed))
            )
        }
        (1, _) => {
            let axes: Vec<Axis> = widths.iter()
                .zip(Axis::ALL.iter()).filter(|(w, _)| **w != Width::One)
                .map(|x| x.1.clone()).collect();
            let axis = axes.choose(rng).expect("axis vector should have exactly two elements");
            let (low, high) = seed.get_field(axis);
            let mid = rng.random_range(low + 1 ..= high - 1);
            let (low_subseed, high_subseed) = seed.split(axis, mid);
            Tree::Node(
                Box::new(gen_tree(rng, low_subseed)),
                Box::new(gen_tree(rng, high_subseed))
            )
        }
        (0, _) => {
            let axis = Axis::ALL.choose(rng).unwrap();
            let (low, high) = seed.get_field(axis);
            let mid = rng.random_range(low + 1 ..= high - 1);
            let (low_subseed, high_subseed) = seed.split(axis, mid);
            Tree::Node(
                Box::new(gen_tree(rng, low_subseed)),
                Box::new(gen_tree(rng, high_subseed))
            )
        }
        _ => panic!("something wrong with the widths of the axes"),
    }
}

pub fn generate_level(side_len: u8) -> Vec<Block> {
    let len = side_len as i32;
    let seed = Seed { x: (0, len), y: (0, len), z: (0, len) };
    let mut rng = rand::rng();
    let tree = gen_tree(&mut rng, seed);
    let gblocks = flatten_tree(&tree);
    gblocks_to_blocks(gblocks.as_slice())
}
