use bevy::math::*;
use bevy::prelude::{Component, Reflect};
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Component, Reflect)]
pub enum Axis { X, Y, Z }

impl Axis {
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    pub const NOX: [Self; 2] = [Self::Y, Self::Z];
    pub const NOY: [Self; 2] = [Self::Z, Self::X];
    pub const NOZ: [Self; 2] = [Self::X, Self::Y];

    pub fn next_rh(self: &Self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::Z,
            Self::Z => Self::X,
        }
    }

    pub fn cross(self: &Self, other: &Self) -> i32 {
        match (self, other) {
            (Self::X, Self::X) => 0,
            (Self::Y, Self::Y) => 0,
            (Self::Z, Self::Z) => 0,
            (Self::X, Self::Y) => 1,
            (Self::Y, Self::Z) => 1,
            (Self::Z, Self::X) => 1,
            _ => -1,
        }
    }

    pub fn remaining(self: &Self, other: &Self) -> Option<Self> {
        match (self, other) {
            (Self::X, Self::Y) | (Self::Y, Self::X) => Some(Self::Z),
            (Self::Y, Self::Z) | (Self::Z, Self::Y) => Some(Self::X),
            (Self::Z, Self::X) | (Self::X, Self::Z) => Some(Self::Y),
            _ => None,
        }
    }

    pub fn remaining_two(self: &Self) -> [Self; 2] {
        match self {
            Self::X => Self::NOX,
            Self::Y => Self::NOY,
            Self::Z => Self::NOZ,
        }
    }

    pub fn unit_vector(self: &Self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }

    pub fn vec3_component(self: &Self, v: Vec3) -> f32 {
        match self {
            Self::X => v.x,
            Self::Y => v.y,
            Self::Z => v.z,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Component, Reflect)]
pub struct Direction {
    pub axis: Axis,
    pub positive: bool,
}

impl Direction {
    pub fn new(axis: Axis, positive: bool) -> Self {
        Self { axis, positive }
    }

    pub const XP: Self = Self { axis: Axis::X, positive: true };
    pub const XN: Self = Self { axis: Axis::X, positive: false };
    pub const YP: Self = Self { axis: Axis::Y, positive: true };
    pub const YN: Self = Self { axis: Axis::Y, positive: false };
    pub const ZP: Self = Self { axis: Axis::Z, positive: true };
    pub const ZN: Self = Self { axis: Axis::Z, positive: false };

    pub fn sign(self: &Self) -> i32 {
        if self.positive { 1 } else { -1 }
    }

    pub fn unit_vector(self: &Self) -> Vec3 {
        (self.sign() as f32) * self.axis.unit_vector()
    }
}

fn check_overlap_rectangles(rect1: IRect, rect2: IRect) -> bool {
    !rect1.intersect(rect2).is_empty()
}

fn check_overlap_in_direction(b1: &Block, b2: &Block, direction: &Direction) -> bool {
    let (rect1, rect2) = match direction.axis {
        Axis::X =>
            (IRect::new(b1.min.y, b1.min.z, b1.max.y, b1.max.z), IRect::new(b2.min.y, b2.min.z, b2.max.y, b2.max.z)),
        Axis::Y =>
            (IRect::new(b1.min.x, b1.min.z, b1.max.x, b1.max.z), IRect::new(b2.min.x, b2.min.z, b2.max.x, b2.max.z)),
        Axis::Z =>
            (IRect::new(b1.min.x, b1.min.y, b1.max.x, b1.max.y), IRect::new(b2.min.x, b2.min.y, b2.max.x, b2.max.y)),
    };
    check_overlap_rectangles(rect1, rect2)
}

#[derive(Clone, Debug, Serialize, Deserialize, Component, Reflect, PartialEq)]
pub struct Block {
    pub direction: Direction,
    pub min: IVec3,
    pub max: IVec3,
}

impl Block {
    pub fn get_isize(self: &Self) -> IVec3 {
        self.max - self.min
    }

    pub fn get_size(self: &Self) -> Vec3 {
        self.get_isize().as_vec3()
    }

    pub fn get_center(self: &Self) -> Vec3 {
        self.max.as_vec3().midpoint(self.min.as_vec3())
    }

    pub fn from_center_size(direction: Direction, center: Vec3, size: Vec3) -> Self {
        let half_size = size * 0.5;
        let min = (center - half_size).as_ivec3();
        let max = (center + half_size).as_ivec3();
        Block { direction, min, max }
    }

    pub fn get_elongation(self: &Self) -> Option<Axis> {
       match self.get_isize() {
           IVec3 { x: 1, y: 1, z: 1 } => None,
           IVec3 { x: 2, y: 1, z: 1 } => Some(Axis::X),
           IVec3 { x: 1, y: 2, z: 1 } => Some(Axis::Y),
           IVec3 { x: 1, y: 1, z: 2 } => Some(Axis::Z),
           _                          => None,
       }
    }

    pub fn extract_mm(self: Block) -> (IVec3, IVec3) {
        (self.min, self.max)
    }

    fn possible_collision(self: &Self, b: &Self) -> bool {
        let not_self = b != self;
        let diff = b.get_center() - self.get_center();
        let ahead = self.direction.unit_vector().dot(diff) >= 1.0;
        let in_the_way = self.direction.axis.remaining_two().iter()
            .all(|ax: &Axis| ax.vec3_component(diff).abs() < 1.0);
        // info!("possible_collision: self: {:?}, b: {:?}", self.clone().extract_mm(), b.clone().extract_mm());
        // info!("diff: {:?}", diff);
        // info!("(not_self, ahead, in_the_way): {:?}", (not_self, ahead, in_the_way));
        not_self && ahead && in_the_way
    }

    pub fn get_blocks_in_front<I>(self: &Self, all_blocks: I) -> Vec<Self>
    where
        I: Iterator<Item=Self>
    {
        let res: Vec<Self> = all_blocks
            .filter(|b| self.possible_collision(b))
            .collect();
        res
    }

    pub fn get_nearest_block_in_front<I>(self: &Self, all_blocks: I) -> Option<Self>
    where
        I: Iterator<Item=Self>
    {
        let res = all_blocks
            .filter(|b| self.possible_collision(b))
            .min_by_key(|b: &Self| self.direction.unit_vector().dot(b.get_center() - self.get_center()) as i32);
        res
    }

    pub fn move_block(self: &Self, static_block: &Self) -> Option<Self> {
        if check_overlap_in_direction(self, static_block, &self.direction) {
        let length = if self.get_elongation() == Some(self.direction.axis.clone()) { 2 } else { 1 };
        match self.direction {
            Direction::XP =>
                if self.max.x <= static_block.min.x { 
                    Some(Self {
                        min: IVec3 { x: static_block.min.x - length, ..self.min },
                        max: IVec3 { x: static_block.min.x, ..self.max },
                        ..self.clone()
                    })
                }
                else {
                    None
                },
            Direction::XN =>
                if self.max.x >= static_block.min.x { 
                    Some(Self {
                        min: IVec3 { x: static_block.max.x, ..self.min },
                        max: IVec3 { x: static_block.max.x + length, ..self.max },
                        ..self.clone()
                    })
                }
                else {
                    None
                },
            Direction::YP =>
                if self.max.y <= static_block.min.y { 
                    Some(Self {
                        min: IVec3 { y: static_block.min.y - length, ..self.min },
                        max: IVec3 { y: static_block.min.y, ..self.max },
                        ..self.clone()
                    })
                }
                else {
                    None
                },
            Direction::YN =>
                if self.max.y >= static_block.min.y { 
                    Some(Self {
                        min: IVec3 { y: static_block.max.y, ..self.min },
                        max: IVec3 { y: static_block.max.y + length, ..self.max },
                        ..self.clone()
                    })
                }
                else {
                    None
                },
            Direction::ZP =>
                if self.max.z <= static_block.min.z { 
                    Some(Self {
                        min: IVec3 { z: static_block.min.z - length, ..self.min },
                        max: IVec3 { z: static_block.min.z, ..self.max },
                        ..self.clone()
                    })
                }
                else {
                    None
                },
            Direction::ZN =>
                if self.max.z >= static_block.min.z { 
                    Some(Self {
                        min: IVec3 { z: static_block.max.z, ..self.min },
                        max: IVec3 { z: static_block.max.z + length, ..self.max },
                        ..self.clone()
                    })
                }
                else {
                    None
                },
        }
        }
        else {
            None
        }
    }
}
