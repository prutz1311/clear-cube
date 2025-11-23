use bevy_common_assets::json::JsonAssetPlugin;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;
mod block;
mod generation;

#[derive(Resource)]
pub struct LevelHandle(Handle<Level>);

#[derive(Resource)]
pub struct BlockModels {
    pub small_model: Handle<Scene>,
    pub wide_model: Handle<Scene>,
    pub long_model: Handle<Scene>
}

#[derive(serde::Deserialize, Asset, TypePath, Resource)]
pub struct Level(Vec<block::Block>);

impl Level {
    pub fn bounds(self: &Self) -> (Vec3, Vec3) {
        let lower = self.0.iter().fold(Vec3::MAX, |acc, v| acc.min(v.min.as_vec3()));
        let upper = self.0.iter().fold(Vec3::MIN, |acc, v| acc.max(v.max.as_vec3()));
        (lower, upper)
    }

    pub fn center(self: &Self) -> Vec3 {
        let (lower, upper) = self.bounds();
        lower.midpoint(upper)
    }
}

#[derive(Resource)]
pub struct LevelCenter(Vec3);

#[derive(Component, Reflect)]
pub struct MoveDest {
    dest: Vec3,
    should_despawn: bool
}

#[derive(Component)]
pub struct BlockSceneMarker;

pub fn rotate_axis_to_axis(ax_from: &block::Axis, ax_to: &block::Axis) -> Quat {
    match ax_from.remaining(ax_to) {
        None => Quat::IDENTITY,
        Some(axis_to_rotate_around) => {
            let angle = (std::f32::consts::PI / 2.0) * (ax_from.cross(ax_to) as f32);
            Quat::from_axis_angle(
                axis_to_rotate_around.unit_vector(),
                angle
            )
        }
    }
}

pub fn flip_if_necessary(dir: &block::Direction, ax: &block::Axis) -> Quat {
    if dir.positive {
        Quat::IDENTITY
    }
    else {
        Quat::from_axis_angle(
            ax.unit_vector(), std::f32::consts::PI
        )
    }
}

pub fn block_model_rotation(block: &block::Block, models: &BlockModels) -> (Handle<Scene>, Quat) {
    let el: Option<block::Axis> = block.get_elongation();
    let dir: block::Direction = block.direction.clone();
    let dir_rotation = flip_if_necessary(&dir, &block::Axis::X);
    let axis_rotation = rotate_axis_to_axis(&block::Axis::Y, &dir.axis);
    match el {
        None => {
            let model = models.small_model.clone();
            let rotation = axis_rotation * dir_rotation;
            (model, rotation)
        }
        Some(d) =>
            if d == dir.axis {
                let rotation = axis_rotation * dir_rotation;
                (models.long_model.clone(), rotation)
            }
            else {
                let initial_model_elongation = Vec3::Z;
                let pre_rotation = axis_rotation * dir_rotation;
                let model_elongation = pre_rotation.mul_vec3(initial_model_elongation);
                let final_rotation =
                    if model_elongation.abs().abs_diff_eq(d.unit_vector(), 1e-6) { 
                        Quat::IDENTITY
                    }
                    else {
                        Quat::from_axis_angle(dir.axis.unit_vector(), std::f32::consts::PI / 2.0)
                    };
                let rotation = final_rotation * pre_rotation;
                (models.wide_model.clone(), rotation)
            }
    }
}

fn draw_blocks(
    mut commands: Commands,
    level: &Level,
    models: BlockModels,
) {
    let level_center = level.center();
    for b in level.0.iter() {
        let block_center = b.get_center();
        let (model, rotation) = block_model_rotation(b, &models);
        commands.spawn((
            SceneRoot(model),
            b.clone(),
            Transform::from_translation(block_center - level_center)
                .with_scale(Vec3::splat(0.5))
                .with_rotation(rotation),
            BlockSceneMarker,
        ))
        .observe(send_block_on_click);
    }
    commands.insert_resource(LevelCenter(level_center));
}

fn setup_level(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    levelr: Res<Assets<Level>>,
    current_level: Res<CurrentLevel>,
    mut state: ResMut<NextState<LevelLoadingState>>,
) {
    let level = LevelHandle(asset_server.load("level1.json"));
    let small_model = asset_server.load("small_model.glb#Scene0");
    let wide_model = asset_server.load("wide_model.glb#Scene0");
    let long_model = asset_server.load("long_model.glb#Scene0");
    commands.insert_resource(level);
    let models = BlockModels { small_model, wide_model, long_model };

    commands.spawn((
        Camera3d::default(),
        PanOrbitCamera::default(),
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        BlockSceneMarker,
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        BlockSceneMarker,
    ));
    let levelx = Level(vec![
        block::Block {
            direction: block::Direction::ZP,
            min: IVec3::new(0,0,0),
            max: IVec3::new(1,1,1)
        },
        block::Block {
            direction: block::Direction::ZP,
            min: IVec3::new(1,0,0),
            max: IVec3::new(2,1,1)
        },
        block::Block {
            direction: block::Direction::ZP,
            min: IVec3::new(2,0,0),
            max: IVec3::new(3,2,1)
        },
        block::Block {
            direction: block::Direction::XN,
            min: IVec3::new(3,0,0),
            max: IVec3::new(4,1,2)
        },
        block::Block {
            direction: block::Direction::XN,
            min: IVec3::new(4,0,0),
            max: IVec3::new(6,1,1)
        },
        block::Block {
            direction: block::Direction::XN,
            min: IVec3::new(1,0,5),
            max: IVec3::new(3,1,6)
        },
    ]);
    // if let Some(level) = levelr.get(handle.0.id()) {
    //     let blocks: Vec<block::Block> = level.0.clone();
    //     let levelx = Level(blocks);
    //     draw_blocks(commands, &levelx, models);
    // }
    // draw_blocks(commands, &levelx, models);
    let width = current_level.0 + 2; // width starts at 3 from level 1
    draw_blocks(commands, &Level(generation::generate_level(width)), models);
    state.set(LevelLoadingState::Level);
}

fn send_block_on_click(
    click: Trigger<Pointer<Click>>,
    mut commands: Commands,
    mut transforms: Query<(Entity, &mut block::Block, &mut Transform), Without<MoveDest>>,
    level_center: Res<LevelCenter>
) {
    let all_blocks: Vec<block::Block> = transforms.iter().map(|t| t.1.clone()).collect();
    let (entity_id, mut block, transform) = transforms.get_mut(click.target()).unwrap();
    use PointerButton as P;
    match click.event.button {
        P::Middle => {
            info!("block model at coords {:?}", transform.translation);
        },
        P::Primary => {
            let nearest = block.get_nearest_block_in_front(all_blocks.iter().cloned());
            let pos_opt = nearest.clone().and_then(|b| block.move_block(&b.clone()));
            let should_despawn = pos_opt.is_none();
            let new_block = pos_opt.clone().unwrap_or(get_flyaway_block_position(&block));
            if new_block != *block {
                commands.entity(entity_id).insert(MoveDest{ dest: new_block.get_center() - level_center.0, should_despawn });
                *block = new_block;
            }
        },
        _ => (),
    }
}

fn get_flyaway_block_position(block: &block::Block) -> block::Block {
    const EDGE: i32 = 20;
    let block::Block { direction, min, max } = block.clone();
    let size: IVec3 = block.get_isize();
    use block::Direction as D;
    let (new_min, new_max) = match direction {
        D::XP => (min.with_x(EDGE - size.x), max.with_x(EDGE)),
        D::XN => (min.with_x(-EDGE), max.with_x(-EDGE + size.x)),
        D::YP => (min.with_y(EDGE - size.y), max.with_y(EDGE)),
        D::YN => (min.with_y(-EDGE), max.with_y(-EDGE + size.y)),
        D::ZP => (min.with_z(EDGE - size.z), max.with_z(EDGE)),
        D::ZN => (min.with_z(-EDGE), max.with_z(-EDGE + size.z)),
    };
    block::Block { direction, min: new_min, max: new_max }
}

fn animate_moving_blocks(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &block::Block, &MoveDest)>,
    time: Res<Time>,
) {
    for (entity_id, mut tr, block, move_dest) in query.iter_mut() {
        let movement_dir = block.direction.clone().unit_vector();
        let new_translation =
            tr.translation + 16.0 * time.delta_secs() * movement_dir;
        let diff = move_dest.dest - new_translation;
        let dot = movement_dir.dot(diff);
        let should_stop = dot < 0.0;
        if should_stop {
            let mut entity = commands.entity(entity_id);
            if move_dest.should_despawn {
                entity.despawn();
            }
            else {
                *tr = tr.with_translation(move_dest.dest);
                entity.remove::<MoveDest>();
            }
        }
        else {
            *tr = tr.with_translation(new_translation);
        }
    }
}

fn finish_level_if_done(
    mut commands: Commands,
    scene_query: Query<Entity, With<BlockSceneMarker>>,
    blocks_query: Query<&block::Block>,
    mut next_level: ResMut<CurrentLevel>,
    mut istate: ResMut<NextState<Interface>>,
) {
    if blocks_query.iter().count() == 0 {
        scene_query.iter().for_each(|e| commands.entity(e).despawn());
        let current_level = next_level.0;
        *next_level = CurrentLevel(current_level + 1);
        istate.set(Interface::Menu);
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum LevelLoadingState {
    #[default]
    Loading,
    Level,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum Interface {
    #[default]
    Menu,
    Gameplay,
}

#[derive(Resource)]
struct CurrentLevel(u8);

#[derive(Component)]
struct MenuMarker;

fn text(level: u8) -> impl Bundle {
    (
        Text::new(format!("Next: Level {}", level)),
        TextFont {
            font_size: 33.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
        TextShadow::default(),
    )
}

fn button() -> impl Bundle {
    (
        Button,
        Node {
            width: Val::Px(300.0),
            height: Val::Px(65.0),
            border: UiRect::all(Val::Px(5.0)),
            // horizontally center child text
            justify_content: JustifyContent::Center,
            // vertically center child text
            align_items: AlignItems::Center,
            ..default()
        },
        BorderColor::from(Color::WHITE),
        BorderRadius::MAX,
        BackgroundColor(Color::BLACK),
        children![(
            Text::new("Start playing"),
            TextFont {
                font_size: 33.0,
                ..default()
            },
            TextColor(Color::srgb(0.9, 0.9, 0.9)),
            TextShadow::default(),
        )]
    )
}

fn draw_menu(level: u8) -> impl Bundle {
    (
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        },
        // TabGroup::default(),
        children![
            text(level),
            button(),
        ],
    )
}

fn button_system(
    mut commands: Commands,
    mut interaction_query: Query<
        (
            Entity,
            &Interaction,
            &mut Button,
        ),
        Changed<Interaction>,
    >,
    menu_elements_query: Query<Entity, With<MenuMarker>>,
    mut istate: ResMut<NextState<Interface>>,
) {
    for (entity, interaction, button) in interaction_query.iter_mut() {
        if let Interaction::Pressed = *interaction {
            menu_elements_query.iter().for_each(|e| commands.entity(e).despawn());
            istate.set(Interface::Gameplay);
        }
    }
}

fn setup_menu(
    mut commands: Commands,
    level: Res<CurrentLevel>,
) {
    commands.spawn((Camera2d, MenuMarker));
    commands.spawn((draw_menu(level.0), MenuMarker));
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MeshPickingPlugin,
            JsonAssetPlugin::<Level>::new(&["level1.json"]),
        ))
        .add_plugins(EguiPlugin::default())
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(PanOrbitCameraPlugin)
        .init_state::<LevelLoadingState>()
        .insert_resource(CurrentLevel(1))
        .init_state::<Interface>()
        .add_systems(OnEnter(Interface::Menu), setup_menu)
        .add_systems(Update, button_system.run_if(in_state(Interface::Menu)))
        .add_systems(OnEnter(Interface::Gameplay), setup_level)
        .add_systems(Update, animate_moving_blocks.run_if(in_state(Interface::Gameplay)))
        .add_systems(Update, finish_level_if_done.run_if(in_state(Interface::Gameplay)))
        .register_type::<MoveDest>()
        .register_type::<block::Block>()
        .run();
}
