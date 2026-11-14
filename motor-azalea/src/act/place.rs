//! Placing a block into a chosen cell by clicking the face of a chosen support, sent as one
//! use-on packet with that face. Azalea's own interact picks the face under the crosshair,
//! which from many standing spots is a different face than the one that fills the cell.

use azalea::{
    core::{
        direction::Direction,
        position::{BlockPos as AzPos, Vec3},
    },
    protocol::packets::game::{
        s_interact::InteractionHand,
        s_punch::ServerboundPunch,
        s_use_item_on::{BlockHit, ServerboundUseItemOn},
    },
    Client,
};
use reflex_core::math::BlockPos;

/// The face of `against` that touches `cell`, or `None` when they are not neighbours.
pub fn face_between(cell: BlockPos, against: BlockPos) -> Option<Direction> {
    Some(match (cell.x - against.x, cell.y - against.y, cell.z - against.z) {
        (0, 1, 0) => Direction::Up,
        (0, -1, 0) => Direction::Down,
        (1, 0, 0) => Direction::East,
        (-1, 0, 0) => Direction::West,
        (0, 0, 1) => Direction::South,
        (0, 0, -1) => Direction::North,
        _ => return None,
    })
}

pub fn place_at(bot: &Client, cell: BlockPos, against: BlockPos) {
    let Some(direction) = face_between(cell, against) else {
        return;
    };
    let mid = |b: BlockPos| Vec3::new(b.x as f64 + 0.5, b.y as f64 + 0.5, b.z as f64 + 0.5);
    let (c, a) = (mid(cell), mid(against));
    let location = Vec3::new((c.x + a.x) / 2.0, (c.y + a.y) / 2.0, (c.z + a.z) / 2.0);
    bot.write_packet(ServerboundUseItemOn {
        hand: InteractionHand::MainHand,
        block_hit: BlockHit {
            block_pos: AzPos::new(against.x, against.y, against.z),
            direction,
            location,
            inside: false,
            world_border: false,
        },
        seq: 0,
    });
    bot.write_packet(ServerboundPunch);
}
