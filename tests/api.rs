use google_material_design_icons_bin::material_icons;

#[test]
fn direct_access_alpha_shape_is_consistent() {
    let icon = material_icons::android::action::account_balance;
    assert_eq!(icon.width, 48);
    assert_eq!(icon.height, 48);

    let alpha = icon.alpha();
    let expected_len = icon.width as usize * icon.height as usize;
    assert_eq!(alpha.len(), expected_len);
}

#[test]
fn lookup_by_path_is_case_insensitive_and_matches_constant() {
    let icon = material_icons::android::action::account_balance;

    let by_path = material_icons::by_path("ANDROID/ACTION/ACCOUNT_BALANCE").unwrap();
    assert_eq!(by_path, icon);
}

#[test]
fn lookup_by_name_is_case_insensitive_and_matches_by_path() {
    let by_path = material_icons::by_path("android/action/account_balance").unwrap();
    let by_name = material_icons::by_name("Account_Balance").unwrap();
    assert_eq!(by_name, by_path);
}

#[test]
fn all_contains_known_entry() {
    let icon = material_icons::android::action::account_balance;
    let found = material_icons::ALL
        .iter()
        .any(|(path, id)| *path == "android/action/account_balance" && *id == icon);
    assert!(found);
}
