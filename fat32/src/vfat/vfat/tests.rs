use super::VFat;
use mbr::PartitionEntry;
use std::io::Cursor;
use traits::BlockDevice;
use vfat::ebpb::BiosParameterBlock;

#[test]
fn vfat() {
    let vec: Vec<u8> = vec![];
    let device = Cursor::new(vec);

    let partition = PartitionEntry {
        relative_sector: 2,
        sectors: 2,
        ..Default::default()
    };

    let ebpb = BiosParameterBlock {
        bytes_per_sector: device.sector_size() as u16 * 2,
        sectors_per_cluster: 2,
        sectors_per_fat: 2,
        reserved_sectors: 2,
        fats: 2,
        ..Default::default()
    };

    let vfat = VFat::from_inner(device, &partition, &ebpb);

    assert_eq!(vfat.bytes_per_sector, 1024);
    assert_eq!(vfat.sectors_per_cluster, 2);
    assert_eq!(vfat.sectors_per_fat, 2);
    assert_eq!(vfat.fat_start_sector, 5);
    assert_eq!(vfat.data_start_sector, 9);
}
