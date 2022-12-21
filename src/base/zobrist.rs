/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! Hash key generation for boards.

use super::{Color, Piece, Square};

#[inline(always)]
/// Get the Zobrist key for a given key, type, and square.
pub fn square_key(sq: Square, pt: Option<Piece>, color: Color) -> u64 {
    match pt {
        None => 0,
        // Because sq, p, and color are all enums with fixed ranges, we can perform an unchecked
        // get on these indices.
        Some(p) => unsafe {
            *SQUARE_KEYS
                .get_unchecked(sq as usize)
                .get_unchecked(p as usize)
                .get_unchecked(color as usize)
        },
    }
}

#[inline(always)]
/// Get the Zobrist key for a castling right.
/// 0 is for white king castle, 1 is for white queen castle, 2 is for black king castle, and 3 is
/// for black queen castle.
pub fn castle_key(right: u8) -> u64 {
    unsafe { *CASTLE_KEYS.get_unchecked(right as usize) }
}

#[inline(always)]
/// Get the Zobrist key of an en passant square.
pub fn ep_key(sq: Square) -> u64 {
    unsafe { *EP_KEYS.get_unchecked(sq.file() as usize) }
}

#[allow(unused)]
// #[test]
/// Helper function to create the definitions for all the keys in the binary.
/// Prints out source code for each key.
fn print_keys() {
    use super::Bitboard;
    fastrand::seed(12345);

    // player to move key
    println!(
        "pub const BLACK_TO_MOVE_KEY: u64 = 0x{:x};\n",
        fastrand::u64(..)
    );

    // castle keys
    println!("const CASTLE_KEYS: [u64; 4] = [");
    for _ in 0..4 {
        println!("    0x{:x},", fastrand::u64(..));
    }
    println!("];\n");

    // en passant keys
    println!("const EP_KEYS: [u64; 8] = [");
    for _ in 0..8 {
        println!("    0x{:x},", fastrand::u64(..));
    }
    println!("];\n");

    // square keys
    println!("const SQUARE_KEYS: [[[u64; 2]; Piece::NUM]; 64] = [");
    for _ in Bitboard::ALL {
        println!("    [");
        for _ in Piece::ALL {
            println!(
                "        [0x{:x}, 0x{:x}],",
                fastrand::u64(..),
                fastrand::u64(..)
            );
        }
        println!("    ],");
    }
    println!("];");
}
pub const BLACK_TO_MOVE_KEY: u64 = 0x3440_f9f4_6981_0c7b;

const CASTLE_KEYS: [u64; 4] = [
    0xc794_9c1f_4870_8594,
    0xda98_0e92_2b5f_67f8,
    0xc67a_4ef3_3e3b_4c59,
    0xa2cb_0d86_5391_3e79,
];

const EP_KEYS: [u64; 8] = [
    0xe3ea_bfc9_f768_dfe4,
    0x310e_f8ad_e9f0_8fcb,
    0x54cf_e575_ef62_4331,
    0xa1c4_63f4_5c9e_614d,
    0xaed7_4c12_d7dc_5549,
    0xa85b_107d_2b3b_4d36,
    0xea99_6334_c5a4_4d00,
    0x93a6_73df_a52c_8f98,
];

const SQUARE_KEYS: [[[u64; 2]; Piece::NUM]; 64] = [
    [
        [0x7a8f_6f07_a994_160f, 0x9697_1d2f_b6c9_117d],
        [0x680c_9e02_4eab_9c67, 0x7a88_4882_d56e_d146],
        [0x1b6a_c3b8_d9d2_3d69, 0x5068_5cca_fd89_5193],
        [0xb80e_3031_2851_6329, 0x9644_8615_50ac_da55],
        [0xa57c_950c_3536_2588, 0xfc5e_06a6_1d1f_a511],
        [0x900f_bd55_461c_2c1d, 0x6e41_dbcb_c676_8190],
    ],
    [
        [0xa69b_813d_4490_5e46, 0xc1d0_9ea5_2bcd_ce99],
        [0x32f9_9b12_4304_e140, 0x7c9d_9683_0f6f_b137],
        [0x95e1_abd7_665a_f1b3, 0x771f_1e1c_e10f_776a],
        [0x13d0_1466_db4e_91e4, 0xde6c_eead_0836_4dc3],
        [0x1caa_0ade_a80e_8826, 0x0ddf_5132_5de5_4603],
        [0x8838_438b_9ebd_c1cc, 0x151b_0c8a_168e_8fbb],
    ],
    [
        [0xdbf6_cb75_6fea_23ff, 0xa9a7_cea4_c9fd_e245],
        [0x7b7a_f0b6_6f15_f1c2, 0x62b3_4798_cfb8_c4d4],
        [0x06b2_eee3_b598_3c3b, 0x13fd_9a7a_d020_0318],
        [0x9955_27b3_54ed_86e1, 0x8038_de05_a2ad_86e2],
        [0x87a5_a9ac_0a5e_22fb, 0xbcf6_437a_8f51_4fba],
        [0xa90b_eed1_b245_3f2c, 0xd808_464d_4c08_c6fc],
    ],
    [
        [0x97a0_7dd9_59f7_c759, 0xb630_8f00_6986_16fb],
        [0x65fa_a808_87e3_98a3, 0x6c52_3bb7_7d1a_0683],
        [0x3713_7e93_ecd7_c3c6, 0x1a27_a7a4_3ee9_ea29],
        [0x3e15_91aa_273e_e489, 0xda0f_fa53_c91d_e2d2],
        [0x7310_5088_9420_2253, 0x5fa9_dfbc_d51f_aa9b],
        [0xb891_a9c6_b48a_eb37, 0xbb1f_f48a_a2ae_b552],
    ],
    [
        [0xfcb9_a67f_6d03_9b97, 0x0d5c_b07e_3fb9_5c3d],
        [0x504f_4591_78be_586f, 0x7d04_84cf_3bdb_8de2],
        [0xc1ef_4ce6_52d2_5650, 0x081e_b965_eda3_03b3],
        [0xd37e_d6cb_4e41_98c9, 0x2ed3_aa19_15d2_d71e],
        [0xc43f_2fb7_ff2c_c63d, 0x2ce1_e0f8_9a46_578b],
        [0x1439_71cf_0420_126b, 0xb27f_2aa6_6e21_bec0],
    ],
    [
        [0x5b1a_3a75_cca3_1163, 0x40bd_2acc_7e56_5da2],
        [0x8998_8be8_7f2d_1b64, 0x85f3_b74a_c08e_81d2],
        [0xc684_addf_f3eb_f661, 0x416e_caf7_adfa_083f],
        [0x056b_d3bf_7d7d_c7f6, 0xefb8_e59b_6ef3_e6fd],
        [0x1324_3cff_2d98_6ca3, 0x845b_b801_7aea_67a7],
        [0xee0d_5b86_fcdc_dbe1, 0x9cf8_ccf6_1f84_a344],
    ],
    [
        [0x4e2f_0d22_2534_5789, 0x74bd_e80f_46fd_f4d7],
        [0x111c_67e2_4d3b_1c5e, 0x400c_e3e4_a05b_fac2],
        [0xbd0b_9ccb_bd3f_7b9e, 0x8ae6_563c_6198_76b7],
        [0x8b83_0837_ff99_fe1f, 0xe4a1_f554_e741_6a2b],
        [0xf1b3_8bcc_a4ea_c996, 0x322a_186f_3b3c_ae3c],
        [0x88b6_31f7_09b6_0100, 0x852e_f553_3dcf_beb9],
    ],
    [
        [0x9f3f_9678_9d2d_8ed8, 0x4571_5398_e06f_070d],
        [0xbcd9_2c1b_9640_ecfd, 0xb40f_f376_175e_fa7a],
        [0xb640_a630_2bc7_c492, 0x0982_2156_d312_33d3],
        [0x0b26_a678_b7e1_1faa, 0x3551_6b54_b7a9_6d40],
        [0xdc02_a071_ec19_1052, 0xdf17_9271_ef16_8a9c],
        [0x4253_b34c_3a66_2b8f, 0x8710_541e_d439_a319],
    ],
    [
        [0xd37a_2dd4_8341_d993, 0x9ece_adf3_edec_2132],
        [0x9174_f7a2_0420_0d9b, 0xa657_d350_1e49_2ec6],
        [0x9195_ff11_76c8_1dfa, 0x0521_2c7c_0e7f_460f],
        [0x57fc_52ae_68e3_c687, 0x9112_e28c_0f7d_a82b],
        [0x2c60_fbf9_957d_8993, 0x75c1_3c39_5137_0ebe],
        [0xd6b5_ff65_53f6_f7d8, 0xa422_8d19_d039_00c0],
    ],
    [
        [0x2661_31ef_db13_6212, 0xcd4c_040f_8eec_83a0],
        [0xd3ce_df31_423e_ea9b, 0xda0b_c131_a38c_ee08],
        [0x3cd8_3daf_4088_68c0, 0xfa50_eb10_cc19_8806],
        [0x5823_85f6_4201_993e, 0x5a17_51fa_a7ea_6dfb],
        [0x0f17_7cc2_105e_e395, 0xf008_3a33_8bef_5d6f],
        [0x40f7_1955_4742_457f, 0x0b04_72a8_1ca5_208e],
    ],
    [
        [0xaafa_81f5_edd1_2f58, 0x14e3_ece2_f7e1_59be],
        [0x4224_c06c_f161_dc78, 0x6989_15b3_ea06_e751],
        [0xef1b_9ccb_2509_d0de, 0x04b6_668b_43e0_2003],
        [0xefbc_a72f_1902_2052, 0x5d0f_96b5_53f6_de07],
        [0xa573_2041_d124_50aa, 0x49cc_4ef4_6cef_0608],
        [0x53ec_2517_e5c3_af16, 0x08bc_6583_d421_3f6f],
    ],
    [
        [0x0a7e_e1a7_a249_3cb7, 0x86d2_8f5b_490c_e47f],
        [0x2d5b_e244_c729_8d63, 0x3ead_bb3b_eb95_1085],
        [0x190c_6b89_c409_f038, 0x2a1f_3f23_d023_979c],
        [0xdd3b_8c7d_0e7f_ddbd, 0x5454_68da_9697_f4de],
        [0xfa93_78b8_36a9_7d22, 0xf9d3_c6ed_5cca_e44c],
        [0xa6ff_4ec1_ff37_5b74, 0xbd3f_f736_8ca4_9b4d],
    ],
    [
        [0x45ed_4d8b_8532_8a9a, 0x8d7d_5fba_2911_359d],
        [0x9b5b_af08_91f4_644c, 0xf99f_2c7d_fdcd_71d5],
        [0xbc1b_8e8b_389a_6337, 0x4a5d_2537_40d3_38fc],
        [0x3493_42f6_719f_d484, 0x95b9_fb70_f361_ab9e],
        [0xfdfd_1396_2981_8cda, 0x8be9_e02a_cabf_0add],
        [0x52a1_7ff7_5527_b983, 0x7355_4752_e294_229b],
    ],
    [
        [0x8cee_e843_954a_8b39, 0x61fe_ef89_5935_6ba8],
        [0xcba3_1190_0f45_8d6e, 0x887b_ef90_3a8a_9efd],
        [0xbcf6_8b86_f9a8_8691, 0x12eb_6604_23d6_adad],
        [0x6600_2362_a4f6_4015, 0x2dfa_6fc9_3619_3b9d],
        [0x735b_25ae_6078_79e7, 0xcee5_3370_91a9_bed2],
        [0x672a_c937_cb86_cb3f, 0x54d7_a183_8da2_a346],
    ],
    [
        [0x3074_e881_c901_cb89, 0x3706_82ba_a40e_9153],
        [0x4303_d814_1f7a_1f98, 0xdf53_3de5_2612_fef4],
        [0x679a_b215_e8c9_6b9d, 0xe2e6_e3f2_6dec_3b75],
        [0x3890_4a4d_38fb_a812, 0x4588_77b3_5c5e_3003],
        [0x70cc_d3fb_07f9_425d, 0x6ab1_be97_8917_0608],
        [0x4c7c_feca_b108_1028, 0xce45_e514_1b46_616a],
    ],
    [
        [0xca96_f491_9828_c9f6, 0xaa28_d70a_046b_3432],
        [0x5f47_45dd_e035_4540, 0xd7fb_5155_8d22_de75],
        [0x1724_dc96_0d14_841b, 0x48c4_fcc6_db91_8d31],
        [0xbd8d_8043_9711_46c7, 0xfcd2_aac4_8028_0b4d],
        [0xc3a7_50d8_e189_47f1, 0x83ab_3967_d488_4600],
        [0x83f3_1dc6_84ff_1004, 0x4d4e_b1c2_9fbb_5464],
    ],
    [
        [0x1874_78d2_7e02_4511, 0x28a1_91a1_649b_4c92],
        [0x1178_499a_d64a_9cde, 0x8cac_3b37_db05_c4c3],
        [0x7eab_d34f_5a47_e176, 0x0b57_83e3_7c3c_2a09],
        [0x707a_03e1_c93a_0111, 0x2764_6f40_77a5_f4cc],
        [0x083e_7e03_06c8_6094, 0xb3d9_d759_53f0_8e40],
        [0x0efd_713d_83cd_33a9, 0x4e29_aa68_8fea_04db],
    ],
    [
        [0x84c1_b730_4133_83dc, 0xec10_13ce_a128_9e69],
        [0xf993_c4e4_8b29_e8f6, 0x3e3b_70bb_3a34_55b3],
        [0x2c24_fc28_7122_d7dc, 0x8f89_5196_3b86_c052],
        [0x1f05_cc3d_0e09_1d9f, 0xa470_8326_d9de_b2df],
        [0x9a72_6c09_2972_dba1, 0xb59e_6430_cf16_9554],
        [0xc9f5_8854_ee47_06d5, 0x4336_2c65_5a2c_6b11],
    ],
    [
        [0xd1b1_691c_01a0_e47a, 0x847f_9e8e_e82f_d8e2],
        [0xe516_271c_bc34_a74b, 0x25f1_377d_edaf_07b3],
        [0x2339_173e_20a1_b285, 0xc3d7_c36a_dd10_b1f5],
        [0x6d22_dc15_737f_143a, 0x3cd2_3042_88b2_0e45],
        [0xf24b_4504_c56c_ef6d, 0x8caa_ad65_c855_11aa],
        [0x6bf6_a021_9d48_682f, 0xe090_03fd_22c2_6aa5],
    ],
    [
        [0x38cd_23a8_50aa_24a8, 0xd74b_0f35_312a_78c5],
        [0xab7d_9f31_7ba5_1067, 0x8f9d_cea2_56c4_9670],
        [0x34d4_61a4_44fe_dd1e, 0x133c_1efc_4ce0_41ae],
        [0xceb3_c445_be25_db69, 0xd2d0_b38f_54d0_4efb],
        [0x028d_1584_81d0_aa43, 0x520d_c1ab_45bf_57ce],
        [0x87c1_a9fa_8377_8629, 0x169e_e8ce_7ed4_7808],
    ],
    [
        [0xd3a6_15f0_e7c2_30f4, 0x08c7_0b64_deba_4e91],
        [0xab25_303f_82e6_a964, 0x77c6_4fdb_d575_4840],
        [0x7356_a175_f849_6ab2, 0x0b5a_9b49_2995_4c30],
        [0x26f7_5de5_0fa3_cbc2, 0x51e0_19ae_1942_05c8],
        [0x09be_4e46_9187_6b69, 0xf62b_255e_14fc_2cec],
        [0x01d2_160f_6120_60d3, 0xc473_c4c5_9d8d_7c36],
    ],
    [
        [0xda1b_d073_2e5a_a04b, 0x1643_dfc2_bd4a_f914],
        [0x142e_1808_a156_4beb, 0x8187_ea0d_7b21_cfb1],
        [0xba53_765e_8ed2_aa8c, 0x6a3c_edd3_2efc_0a76],
        [0x3ca8_a243_ac28_0196, 0xe3f4_018e_b62b_4425],
        [0x170e_d952_8583_da6e, 0x04be_b4e6_4471_eed8],
        [0x3234_4d0b_47ef_8374, 0xda23_edb8_2e6a_d862],
    ],
    [
        [0x01eb_6c06_977d_4930, 0x7cb4_78ae_aee8_e888],
        [0xfd23_90bd_2044_82a8, 0xe0a3_18eb_6bac_6f58],
        [0xe4a5_67ce_17dd_cf3e, 0x47b6_6e75_bcd1_fcd5],
        [0xcc40_31c6_33fe_9936, 0x855a_0511_5b3c_87ca],
        [0x8a66_bc8a_a78f_f0ec, 0x4bdb_cda1_7a34_770e],
        [0xf6c6_2ca8_d8a0_0ac7, 0x7d46_046a_2a8a_d518],
    ],
    [
        [0x61e2_a359_409e_e9aa, 0xc9cf_5b72_3233_b23c],
        [0x9c0a_c72a_cae4_e975, 0x481e_414c_da02_b493],
        [0xfae6_aa6a_b2aa_f490, 0x0aa9_3ad2_ae0a_1307],
        [0x4d8c_0314_9008_552a, 0x07d2_8061_5ec9_8e04],
        [0x901c_974d_a3b9_02df, 0x1ffc_4c28_0166_63f5],
        [0x6dcd_0ea3_f2d6_3836, 0xf97a_0252_ad0b_a1ed],
    ],
    [
        [0xc30d_1b1d_918f_d5f1, 0xcaa6_6a2d_ece4_d03e],
        [0x43ef_7971_e718_c496, 0xa338_6a6f_ca45_4f4f],
        [0xc9b0_3eec_20e0_c286, 0xbfc2_d913_dbc5_ade7],
        [0x2350_8aff_d535_eb91, 0x8943_e4fc_31cc_544e],
        [0x524d_1416_7391_8a03, 0x68ae_ec09_597c_62e5],
        [0x710c_3b31_6bd0_285b, 0x5eca_36b8_3849_d8e6],
    ],
    [
        [0x3ae2_ee81_75d9_52cc, 0x68c9_849c_4e26_cd93],
        [0x0f1a_d111_f7df_2f71, 0x1a26_1eda_0b12_8113],
        [0xbace_0903_ef86_cc3a, 0xdefd_4d83_367f_4878],
        [0x598c_9d72_5ed2_6963, 0xc8ca_2179_1e8b_59aa],
        [0x219b_4553_b26b_30f1, 0xd30c_e653_40a1_2748],
        [0x3d86_a89b_7f75_07d7, 0x8473_9e8d_9dc8_f4f9],
    ],
    [
        [0xda69_cada_132e_8a72, 0x4d41_5989_2c1c_940a],
        [0x61ac_fca1_af31_e90f, 0xb58e_b64b_b9d4_6c6a],
        [0x2832_799d_8f37_7a27, 0xea4b_d591_5257_8143],
        [0x7cf3_3d89_8d95_77c8, 0xdee7_deee_13c4_8dce],
        [0xe5b6_c151_2870_ee0f, 0x51f3_8347_48b3_cc73],
        [0xb3eb_92b4_085d_fab7, 0x3b4b_5bdd_70dc_3dd6],
    ],
    [
        [0x0ba0_bc25_2376_696a, 0xda6d_e87f_0742_1176],
        [0xd831_4d64_b50b_f5b5, 0x8e11_70c3_acf0_d427],
        [0x8221_c083_02c6_d485, 0x116f_ba9a_0e0e_9174],
        [0x62b9_6529_1d80_ac65, 0xbd0b_6de4_6418_a8b7],
        [0x5853_ac8e_4193_01c4, 0xb4f3_4f1a_130f_a07f],
        [0x827e_deed_15e3_f9a6, 0x6364_71d0_ff6e_b4b1],
    ],
    [
        [0xcb4c_d1ec_aad4_a2e9, 0x61c7_7727_771e_ff07],
        [0x20cb_f8a9_bbea_93d4, 0x5d92_428e_a18f_15f8],
        [0x7fa3_cf35_44e5_3b2a, 0x75fd_87c5_b0f4_6996],
        [0xfc5a_37fe_3971_af69, 0xa6a8_17e7_0450_d6f5],
        [0x73cb_f593_44f4_ec8c, 0x1919_f4ac_8496_3e5a],
        [0x8e50_5293_70a7_f70c, 0x39cf_b61a_bed7_786e],
    ],
    [
        [0x9044_2ddb_8391_8525, 0x35b8_e89e_9da4_4589],
        [0x22fa_215f_cb37_7795, 0x07ba_6991_ccd6_a751],
        [0x419f_e3ae_a081_f59c, 0x4a7c_cae1_3051_30fe],
        [0xaebe_4b64_311e_a941, 0xda7f_cc38_deb5_e2f6],
        [0xea06_d6b2_0582_17b8, 0xf375_d00e_a14a_4c90],
        [0x13b9_4b07_83fc_1620, 0x6c74_7a43_5417_bb9e],
    ],
    [
        [0x2405_d49b_83c3_58c9, 0x7c4b_5a4f_5ce0_574f],
        [0xd57b_1c8d_327a_877b, 0x597e_fc75_fa0c_5613],
        [0x0f02_1554_d8d9_d6da, 0xc99a_32a8_3342_15ca],
        [0x417d_d5f0_6c25_b117, 0x5d80_0dd1_edbd_f309],
        [0x077c_c780_26c6_7dd1, 0xf24d_f479_ca78_f984],
        [0x56ce_61b7_5eed_86dd, 0xe5f4_3158_b018_8bca],
    ],
    [
        [0x9f97_0f5c_0d07_e26a, 0xfb69_61b4_2ef9_9f01],
        [0xbf9c_7122_4e27_a3c8, 0x6524_2a20_ea7b_b6d4],
        [0x42b8_fd06_6adb_5307, 0x7863_9b84_b9d0_3a71],
        [0x2eb1_71b4_f090_4ef8, 0xcb25_2136_e953_21e3],
        [0x37a2_cb67_1dca_69ca, 0x7205_c0b6_5b1d_48f2],
        [0x8a73_8ec5_abcd_a1c4, 0x2b5c_94cd_2f46_1e31],
    ],
    [
        [0xef47_0310_0ae9_2078, 0xc4f6_33a5_81e5_63b5],
        [0x6fff_a9c4_d077_1fc0, 0x1658_1521_8b91_c3f1],
        [0x97b0_eb4b_6e09_8dfd, 0x2b8b_6d7f_920a_9b91],
        [0xefd8_5558_9ccb_305b, 0xab11_8a0e_9751_96fc],
        [0x45bf_b0e8_37d1_a910, 0xdeaa_6087_7ad6_ddf7],
        [0x1838_707c_c193_66e6, 0xa367_9ad5_1c30_6f4f],
    ],
    [
        [0x36bf_7888_664d_7536, 0x2e64_3f48_3340_58b4],
        [0x55ba_a624_c9bd_1a8b, 0x635b_7591_62b1_ca05],
        [0x556b_d61c_3980_0179, 0x1d48_9999_2030_f195],
        [0xfc18_7527_8609_5734, 0x17cd_0791_1acb_4066],
        [0xb299_ae74_e759_5e32, 0xa47f_6170_c3f6_2ebe],
        [0x8725_f347_0439_4e45, 0xca8a_8277_c0a9_0e10],
    ],
    [
        [0xbb1b_7111_b8c9_33c9, 0x2f2c_4fa8_2de9_89d3],
        [0xbb05_67bd_2522_9cd8, 0x51dc_207b_bc27_7384],
        [0xfd47_1cb1_f346_d8b0, 0xebe5_562c_17b5_1684],
        [0xb742_1899_e2a4_7d1d, 0xf60a_f440_d9e8_4c57],
        [0x1349_569d_5679_a0ba, 0xda63_d6cd_d06e_a38d],
        [0x2de3_51b0_96d9_31c6, 0x343b_9467_4d6d_18b4],
    ],
    [
        [0x7b70_6fdb_a000_2431, 0x5fd3_6406_32f7_6f05],
        [0x5299_0934_0886_0b16, 0x6cee_2321_a624_9e60],
        [0x061e_d8e4_07c9_e1ae, 0x9007_f463_debd_9548],
        [0x40f5_edaa_ea63_46f4, 0xc15d_8c30_b3e5_dffe],
        [0xa43f_6730_daa3_1c50, 0x53c4_23c3_3e7b_2dce],
        [0xb944_16e5_0862_e4ec, 0x4063_8a05_5d80_3886],
    ],
    [
        [0xac6b_6122_c645_d80d, 0x8cc9_c4be_8bca_183b],
        [0x8921_e0e8_872b_5fb7, 0xccc4_be24_92eb_a3ca],
        [0x8e8d_c42c_8ed7_e488, 0xc4f5_6b8d_00c3_d938],
        [0x49a1_e378_34b1_f0be, 0x3501_f85d_c248_033a],
        [0x42f3_a782_cfca_3eb8, 0xa8f1_15c8_ac91_fea2],
        [0xbfe5_90b2_b31b_ad9c, 0x1297_cb92_74a7_d462],
    ],
    [
        [0x4470_48fe_8213_79ec, 0xfca2_37a4_046d_37ab],
        [0xbcc6_9293_df97_fd8f, 0x35a2_9259_0a06_261e],
        [0x5370_b765_c915_9a3a, 0x80f5_9279_e78a_56d0],
        [0x249c_9617_151b_fd8e, 0xb8e3_cb15_ebcb_be9e],
        [0x4f46_90b5_b510_9f3d, 0x715b_2f68_3561_abe7],
        [0x838b_bc1f_ffcd_4535, 0x6726_49b6_fb2c_824c],
    ],
    [
        [0x75f2_4526_b296_b799, 0x2271_5315_4f7c_7953],
        [0x1634_e086_2331_1125, 0xa426_22f5_4c5a_5b6c],
        [0x0ba7_f3fe_2e34_cd73, 0x2f5b_ccfe_50b7_f612],
        [0xff13_d109_9870_5623, 0xae29_29e4_5b4d_41ea],
        [0x8eef_356f_e6c0_dd1a, 0x8de2_18d1_d350_ce65],
        [0xd4a9_a89c_59dd_af0e, 0x8c97_f28a_3914_11bb],
    ],
    [
        [0xe88a_d6e0_6665_0504, 0x6955_4b0f_7f3e_f62e],
        [0x1c95_df7c_590a_7570, 0xb1b4_d038_365d_cb77],
        [0xf67f_453d_7219_3959, 0xe656_ae57_4431_94a3],
        [0x47c4_794f_e657_9033, 0xc44e_5048_6eb8_56e1],
        [0xb7a4_5d3d_8087_6154, 0x5c05_ef19_eaa8_6da4],
        [0x3881_66b7_137e_4d28, 0x4ccd_3647_bdfe_9219],
    ],
    [
        [0x9626_9ab8_1c40_1993, 0xd3ea_458d_fdbb_721a],
        [0x537b_b5bb_08c9_de63, 0x7ecc_ad15_bc61_0089],
        [0xe3cf_085b_7829_c594, 0x252b_7d28_f6cf_b83d],
        [0x9789_8edc_6796_a0d8, 0xa600_ae14_670b_c982],
        [0x905b_10e7_8f85_24b8, 0x86e1_e542_8b5f_7a43],
        [0xf5f2_6c22_36dc_56ba, 0x6460_18dd_95bc_6c06],
    ],
    [
        [0xf3b7_6f0b_0941_f5de, 0x9d31_86ea_d25f_318c],
        [0x850a_8a30_7a88_c52b, 0x7f1e_fe4a_6755_54f4],
        [0x4907_3060_ae97_c30c, 0x3909_64e3_5776_6b47],
        [0xa7cc_4de4_e73e_5181, 0x3cc5_a49c_c869_de26],
        [0x1490_de3b_87a6_a9a3, 0x9a07_ed52_472a_01bf],
        [0x8cb1_e0b0_fabb_decc, 0xf8b1_7ed2_b88d_a5d5],
    ],
    [
        [0x91f8_98d2_0bdf_72db, 0x1d91_c1d4_cdb6_803d],
        [0x3304_7b50_6228_27df, 0xe170_1e5c_8abd_31ef],
        [0x5840_8f17_0025_3477, 0x6a71_f2fa_6034_7f00],
        [0xfa32_6070_8b70_42f5, 0xf888_8876_687e_3dcf],
        [0x4cc0_588b_77c8_70ed, 0x7e8e_7e56_0d44_cf51],
        [0x7f6c_c29e_c6bd_fd87, 0x8acb_1076_6d57_8abd],
    ],
    [
        [0x3792_16d2_6f0f_6048, 0x50e0_8bd2_a68c_1dac],
        [0x54cd_97df_253a_b1f5, 0x7807_3dfc_36fd_ed40],
        [0xc0b7_d3ae_0327_e8ab, 0x5b53_df47_a180_d2c2],
        [0x2eb6_e2ca_d582_aa93, 0x98a5_4917_b339_c33a],
        [0xd1a1_c9f9_877c_8b7b, 0xad56_fa67_83ed_d6c3],
        [0x5b10_951f_ecb5_7fd4, 0x7cc6_7e42_6895_f308],
    ],
    [
        [0x0e65_c0a3_aade_74f7, 0xf0bb_8a54_6044_04c8],
        [0x86e3_0f3e_4db2_cba2, 0x6144_07ae_ec67_80fb],
        [0xdcb5_b540_f353_2d95, 0x66d0_4c70_da93_9725],
        [0x7b99_4039_bdea_1697, 0x4c1e_f45c_348f_09be],
        [0xe653_e6bb_f4b3_cf4b, 0x7c9c_6bfa_44a3_4ec3],
        [0x928c_36c8_a3e0_f7bf, 0xab7e_e787_c1e9_3d2b],
    ],
    [
        [0x012d_9cfc_78f6_de64, 0x37aa_3df1_e139_8e33],
        [0xf068_39d8_d875_6ddc, 0x455f_e3f0_840c_2f71],
        [0x6951_e856_c508_4973, 0x834e_7602_7c48_8519],
        [0xfd62_ea30_e7ab_1445, 0x11ab_e03e_aec4_d1d4],
        [0xab08_1849_1360_7c21, 0x6769_b7fe_3942_2972],
        [0xcf62_88ee_b934_dd19, 0x255f_da01_6a46_e4d0],
    ],
    [
        [0x0609_0669_2892_b1cf, 0x27e5_0b56_f707_d988],
        [0xeab9_6715_df72_f3c0, 0x4213_ea68_1e38_779b],
        [0x12bc_cd84_0f91_d237, 0x4107_d5f4_cbe7_fc16],
        [0xb09e_24a6_a8de_4571, 0xeafb_6cb3_7213_8b7a],
        [0x5474_20d5_0136_b284, 0x77d7_a570_b578_3f55],
        [0x2d53_68a6_5320_6955, 0xb8ee_a423_ede6_b703],
    ],
    [
        [0x1bea_9718_41cb_225b, 0x282e_cf99_a4c7_97eb],
        [0xee4d_985f_7712_499b, 0x69b5_d652_596a_304e],
        [0xc28d_dd5e_b95a_97df, 0x8c7b_4022_fd30_5c2f],
        [0x398e_79c9_252d_c4d9, 0x4bb4_3a91_cf88_97df],
        [0x843c_2dda_c0dc_9347, 0x463f_df8a_f13a_65e8],
        [0xd1d4_a808_88b7_4535, 0x87f0_5763_0d89_0774],
    ],
    [
        [0x493e_a26c_20cd_2f1d, 0x53e1_7a28_d1a9_3e81],
        [0xf30f_25c5_9961_34f5, 0xd938_2e3b_f8ad_455f],
        [0xb997_63a8_a7a3_2826, 0x371f_437e_0d1e_7d08],
        [0x96ff_aa55_a90b_102d, 0x2c18_e3df_652d_7897],
        [0xb3ff_c448_642c_9326, 0x8b18_2cc7_2971_8d8f],
        [0x7b1f_53ee_f9a0_c8ee, 0x2949_5791_4656_2f4f],
    ],
    [
        [0xc36d_52f8_a625_7727, 0x9a35_1bed_89e4_94c7],
        [0xb554_d3be_bcab_b14f, 0x3ffd_8cdc_0d0c_59f1],
        [0xd240_63d6_bd71_ea9b, 0xd90c_b610_196b_6e64],
        [0xfee4_1c99_d009_ca9e, 0x72ee_fce8_fb3a_4c93],
        [0xde14_b70e_1ec0_bb47, 0x101c_d909_edfa_eab7],
        [0xc81e_7c5f_5ca6_b678, 0x7f5d_abdd_e679_4be4],
    ],
    [
        [0xbd38_1938_5232_513b, 0x8fca_6552_1cb5_77ec],
        [0xa526_4c80_1ad8_e0a1, 0x541c_e792_213a_32da],
        [0x005e_a416_0e1b_2441, 0xfbee_f565_e46c_84e0],
        [0x502c_ce6a_f991_d907, 0xe732_d589_c457_fbd2],
        [0x0145_5140_d735_b101, 0xce74_824b_94a5_2407],
        [0x0aaa_7acc_08bd_6130, 0xb083_c9b1_3ed7_a562],
    ],
    [
        [0x69c3_2c20_c13a_a19a, 0x7591_bd18_af78_576d],
        [0x3e03_98a6_1e23_a3a4, 0xb936_0944_46ec_8f16],
        [0x6818_c9eb_6450_3827, 0x54f3_be9b_659f_6c3e],
        [0x33f4_a481_5040_73cf, 0x3363_388a_bc11_6cb2],
        [0x198e_2835_ec3a_4d40, 0x4437_2642_b0a8_b855],
        [0x6308_f8e1_c60a_0719, 0x628b_e4f9_d13e_de06],
    ],
    [
        [0xe82d_15c7_4acb_9af0, 0x194d_14e5_7990_4b58],
        [0x7299_58fc_b6d3_3bfe, 0x7a0e_6e0b_2864_da4f],
        [0x8387_e83b_b1ad_27ce, 0xa57f_ad83_9f9a_c467],
        [0x6139_d97e_4038_f578, 0x8646_4a00_bce1_fe52],
        [0x310e_e7cb_6eb6_eda4, 0xd750_3626_ec8d_42ec],
        [0xc4ea_4c91_8675_e9ee, 0xd1f2_ac58_b6c1_777e],
    ],
    [
        [0xebd2_3e84_dcd4_5be3, 0x7fb6_1226_2f4e_22d6],
        [0x2e68_4ff1_8c4a_6a44, 0x0533_a2db_6773_648b],
        [0xbe88_4594_d9d5_98a0, 0x240e_ec45_e24f_2bb4],
        [0x6f98_c8b3_0f55_12af, 0x2c4f_66fa_ef3b_54d5],
        [0xa378_0620_eefa_d115, 0xcf4c_80a2_506d_d1e8],
        [0xec19_27fc_5cfe_36ea, 0x3ab5_f47a_dc19_edf5],
    ],
    [
        [0x2acb_fad9_8fa7_ad1a, 0x5513_25d2_bf6e_5605],
        [0x8d4f_0a50_8451_228d, 0xf954_5100_1f32_7d07],
        [0x718a_c32a_f94c_49a5, 0xf9ac_2bed_ee54_22f5],
        [0x3be2_d6da_cc8c_162a, 0x582c_36ab_e320_4e4d],
        [0xf0d7_f62e_5796_33cf, 0xa2bb_b0e4_c6d2_ae40],
        [0x6bf8_d1f3_07ae_f1a4, 0xaaac_c948_16eb_f90c],
    ],
    [
        [0xde40_9c9c_4ba3_42b2, 0x753b_ae59_a966_89d6],
        [0x8b40_f8a0_7d9b_d042, 0x0382_92b7_ad0c_43df],
        [0x8759_668a_dd08_5edb, 0xadb7_95b8_b3eb_f5de],
        [0x501a_5a8c_3d89_8914, 0x5214_1af2_9f00_d1a3],
        [0x8972_a526_9d99_3a40, 0x366c_6807_312e_4a3f],
        [0xb6d4_e6ea_9140_3499, 0x5587_fbf1_10fd_caeb],
    ],
    [
        [0xab84_d334_a5b8_e476, 0xc164_7f80_43fc_d013],
        [0xf640_b5a0_5de6_45f7, 0x032a_ec8e_6703_7224],
        [0x8883_44bc_eb97_ac06, 0xbe49_849c_1562_233b],
        [0xaf7e_5f9c_0563_af30, 0x22ab_9129_dfe8_5979],
        [0xc30c_e92e_9be3_9080, 0x74de_4d11_f174_1521],
        [0x4bca_885b_3c17_301e, 0x7ae1_1a20_5cdc_6ca9],
    ],
    [
        [0x78b9_9483_a750_f8fc, 0x65b1_8d38_1580_6bd8],
        [0x7e04_d4b2_71f6_11cc, 0xb81f_1796_de4b_8648],
        [0x9693_0ca9_72cf_9511, 0x33d5_0ec9_eb84_48a7],
        [0xd6da_a914_982d_3a0e, 0xdd0e_c1f8_abb5_7b80],
        [0x1d49_d49c_e53f_57dd, 0xffb2_9d5a_2afc_02fb],
        [0xec5c_ccb1_f740_eac3, 0x5dbf_e236_d4e5_1d12],
    ],
    [
        [0xf59c_d480_bc7d_94e1, 0xe973_4d1d_ce97_d44f],
        [0x7acf_5094_7552_5eab, 0xe56e_5e92_3671_3a63],
        [0xf21e_7e09_7787_acbc, 0x06df_48f9_3f47_7ba9],
        [0x380a_e49c_a14f_e669, 0x8b0f_9644_546e_24a1],
        [0xa716_43a5_d0bc_aa41, 0x4fd1_3ff7_abb2_6e72],
        [0xe2f8_6e87_9479_7704, 0xce8f_c7ac_cd96_9c8f],
    ],
    [
        [0xcf7d_efc1_6c3b_bb57, 0x72c1_dbd8_6e51_0270],
        [0x5f9d_4008_cb9c_356f, 0x5005_08e3_07cc_4cab],
        [0x6b1e_80c1_27f1_398b, 0xcf99_38c1_f146_005d],
        [0x5a02_a982_f6f2_3e50, 0xf40d_b599_8816_335b],
        [0x3f4b_2de2_094d_6af7, 0x7b95_03a8_37e3_314d],
        [0x8f1e_d221_8718_4dd4, 0x72a9_2fa2_0236_b9d3],
    ],
    [
        [0x5cfc_3d8b_088e_61f3, 0x82f4_74fa_ec03_ae4f],
        [0xe5c4_2f45_1c7f_e808, 0x4643_2c44_ed4d_d9d1],
        [0x8d77_ef8d_dc76_243a, 0xbbff_f772_c8f4_7d86],
        [0x149a_2402_45c8_0665, 0xc844_d823_5d4a_d1f3],
        [0xefcc_ef85_4db4_2050, 0xe337_8c19_8cf7_3ee7],
        [0x9f31_df9d_eefb_5a65, 0x0945_ee6f_5774_0126],
    ],
    [
        [0xee1d_7bd8_af80_64d4, 0xfc76_1b9d_df9f_7e65],
        [0x4868_677c_7d48_34a3, 0x1d72_1985_6907_86b2],
        [0x2a72_33c9_3ad7_b58a, 0xd8a7_6561_b7bd_9015],
        [0xf2e1_39a5_9127_1ed1, 0x4235_1464_af52_0d38],
        [0xec2f_32bc_70a1_b808, 0x2da3_20ba_f9cc_8db5],
        [0xaaa3_da73_dba2_a97f, 0x6667_f494_5b8c_21e7],
    ],
    [
        [0xfc77_622d_57ae_ced3, 0xc834_1f4f_a09c_6321],
        [0x89f2_da02_6fa5_8609, 0xc248_eed4_1fad_0e8b],
        [0x685d_f892_22b5_549f, 0x0319_f713_c0ae_f7af],
        [0xf1cc_59e0_792e_f7da, 0x239c_6ee6_c4f0_43b6],
        [0x9800_e3de_6c97_40b6, 0x8a8e_ac82_6873_0c8a],
        [0xa5c7_6c1e_28cb_f2bb, 0x5fba_9ecb_b932_1165],
    ],
    [
        [0x0f9b_abaf_9432_2bda, 0x0da0_8c4a_e3db_2560],
        [0x7db3_3ec5_f124_1919, 0x2b19_a5a6_922e_6232],
        [0x25b9_c753_98f0_4a16, 0x04f7_36e9_a0ee_a0b4],
        [0x4924_0d0f_1dcf_403a, 0x4fa4_109b_2e31_11a8],
        [0xdfae_f694_3aff_3057, 0x6a1e_dc85_2c5d_62e9],
        [0x45be_48f7_2c52_74f4, 0xaac5_eff0_c18c_c487],
    ],
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that every subset of size `n` of the Zobrist keys do not sum to zero.
    fn verify_independence(n: usize) -> bool {
        /// A list, used for generating all subsets.
        struct Lindep<'a> {
            vector: u64,
            parent: Option<&'a Lindep<'a>>,
        }

        /// Helper function to aid in determining linear independence in the set of Zobrist keys.
        fn independence_help(n: usize, vectors: &[u64], parent: Option<&Lindep>) -> bool {
            if n == 0 {
                return true;
            }

            if n == 1 {
                for &vec in vectors {
                    let mut temp_parent = parent;
                    let mut ortho = vec;
                    while let Some(p) = temp_parent {
                        ortho ^= p.vector;
                        temp_parent = p.parent;
                    }
                    if ortho == 0 {
                        return false;
                    }
                }

                return true;
            }

            for (idx, &vector) in vectors.iter().enumerate() {
                let new_parent = Lindep { vector, parent };
                if !independence_help(n - 1, &vectors[idx + 1..], Some(&new_parent)) {
                    return false;
                };
            }

            true
        }
        let mut hash_keys: Vec<u64> = Vec::new();
        hash_keys.extend(
            SQUARE_KEYS
                .iter()
                .flat_map(|x| x.iter())
                .flat_map(|x| x.iter()),
        );
        hash_keys.extend(CASTLE_KEYS);
        hash_keys.extend(EP_KEYS);
        hash_keys.push(BLACK_TO_MOVE_KEY);

        independence_help(n, &hash_keys, None)
    }

    #[test]
    /// Test exhaustively for linear independence in subsets of all Zobrist keys.
    fn exhaustive_independence() {
        for n in 1..=3 {
            println!("testing for independence of order {n}");
            assert!(verify_independence(n));
        }
    }
}
