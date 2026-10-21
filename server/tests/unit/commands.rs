//! Initially generated with AI assistance on 2026-10-21.

use crate::CommandType;

macro_rules! command_case {
    ($name:ident, $number:literal, $variant:ident) => {
        #[test]
        fn $name() {
            assert!(
                matches!(CommandType::try_from($number), Ok(CommandType::$variant)),
                "wire command {} must map to {}",
                $number,
                stringify!($variant)
            );
        }
    };
}

command_case!(quit_maps_from_wire, 1, Quit);
command_case!(create_channel_maps_from_wire, 2, CreateChannel);
command_case!(display_channels_maps_from_wire, 3, DisplayChannels);
command_case!(open_channel_maps_from_wire, 4, OpenChannel);

#[test]
fn command_discriminants_keep_the_published_wire_numbers() {
    assert_eq!(CommandType::Quit as u32, 1);
    assert_eq!(CommandType::CreateChannel as u32, 2);
    assert_eq!(CommandType::DisplayChannels as u32, 3);
    assert_eq!(CommandType::OpenChannel as u32, 4);
}

#[test]
fn zero_is_not_a_command() {
    assert!(CommandType::try_from(0).is_err());
}

#[test]
fn unassigned_commands_are_rejected() {
    for number in 5..=1024 {
        assert!(CommandType::try_from(number).is_err(), "accepted {number}");
    }
}

#[test]
fn integer_boundaries_are_not_commands() {
    for number in [u16::MAX as u32, i32::MAX as u32, u32::MAX] {
        assert!(CommandType::try_from(number).is_err(), "accepted {number}");
    }
}
