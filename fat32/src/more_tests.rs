mod mbr {
    use mbr::{BootIndicator, MasterBootRecord, PartitionEntry, PartitionType, CHS};
    use std::io::Cursor;

    use std::u32::MAX;

    #[test]
    fn id() {
        let mut buf: [u8; 512] = [0; 512];
        buf[510..512].copy_from_slice(&[0x55, 0xAA]);
        let id: [u8; 10] = [0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
        buf[436..446].copy_from_slice(&id);

        let mbr = MasterBootRecord::from(Cursor::new(&mut buf[..])).expect("valid id");
        assert_eq!(mbr.id, id);
    }

    #[test]
    fn partition_entries() {
        for (i, &offset) in vec![446, 462, 478, 494].iter().enumerate() {
            let mut buf: [u8; 512] = [0; 512];
            buf[510..512].copy_from_slice(&[0x55, 0xAA]);
            let id: [u8; 16] = [
                0x80, 0x00, 0x00, 0x00, 0x0B, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
                0xFF, 0xFF,
            ];
            buf[offset..offset + 16].copy_from_slice(&id);

            let mbr = MasterBootRecord::from(Cursor::new(&mut buf[..])).expect("valid id");
            assert_eq!(
                mbr.table[i],
                PartitionEntry {
                    boot_indicator: BootIndicator::Active,
                    _start_chs: CHS { _chs: [0; 3] },
                    partition_type: PartitionType::Fat32Chs,
                    _end_chs: CHS { _chs: [0; 3] },
                    relative_sector: MAX,
                    sectors: MAX,
                }
            );
        }
    }
}

mod ebpb {
    use std::io::Cursor;
    use std::{u16, u32, u8};
    use vfat::ebpb::BiosParameterBlock;

    macro_rules! test_ebpb_field {
        ($name:ident, $offset:expr, $size: expr, $input:expr, $output:expr) => {
            #[test]
            fn $name() {
                let mut buf: [u8; 512] = [0; 512];
                buf[510..512].copy_from_slice(&[0xAA, 0x55]);
                let $name: [u8; $size] = $input;
                buf[$offset..($offset + $size)].copy_from_slice(&($name));

                let mbr = BiosParameterBlock::from(Cursor::new(&mut buf[..]), 0).unwrap();
                let BiosParameterBlock { $name, .. } = mbr;
                assert_eq!($name, $output);
            }
        };
    }

    test_ebpb_field!(_asm, 0, 3, [0xFF; 3], [0xFF; 3]);

    test_ebpb_field!(oem_id, 3, 8, [0xFF; 8], [0xFF; 8]);

    test_ebpb_field!(bytes_per_sector, 11, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(sectors_per_cluster, 13, 1, [0xFF; 1], u8::MAX);

    test_ebpb_field!(reserved_sectors, 14, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(fats, 16, 1, [0xFF; 1], u8::MAX);

    test_ebpb_field!(max_dir_entries, 17, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(logical_sectors_small, 19, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(fat_id, 21, 1, [0xFF; 1], u8::MAX);

    test_ebpb_field!(_deprecated_sectors_per_fat, 22, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(_sectors_per_track, 24, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(_heads, 26, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(hidden_sectors, 28, 4, [0xFF; 4], u32::MAX);

    test_ebpb_field!(logical_sectors_large, 32, 4, [0xFF; 4], u32::MAX);

    test_ebpb_field!(sectors_per_fat, 36, 4, [0xFF; 4], u32::MAX);

    test_ebpb_field!(flags, 40, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(fat_version_number_minor, 42, 1, [0xFF; 1], u8::MAX);

    test_ebpb_field!(fat_version_number_major, 43, 1, [0xFF; 1], u8::MAX);

    test_ebpb_field!(root_cluster, 44, 4, [0xFF; 4], u32::MAX);

    test_ebpb_field!(fs_info_sector, 48, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(backup_boot_sector, 50, 2, [0xFF; 2], u16::MAX);

    test_ebpb_field!(_reserved, 52, 12, [0xFF; 12], [0xFF; 12]);

    test_ebpb_field!(drive_number, 64, 1, [0xFF; 1], u8::MAX);

    test_ebpb_field!(_windows_nt_flags, 65, 1, [0xFF; 1], u8::MAX);

    test_ebpb_field!(signature, 66, 1, [0xFF; 1], u8::MAX);

    test_ebpb_field!(_volume_id, 67, 4, [0xFF; 4], u32::MAX);

    test_ebpb_field!(volume_label, 71, 11, [0xFF; 11], [0xFF; 11]);

    test_ebpb_field!(system_id, 82, 8, [0xFF; 8], [0xFF; 8]);
}
