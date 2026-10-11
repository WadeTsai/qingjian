//! mmh-reference 的首筆幾何分類測試。

use crate::mmh::first_class;

/// 座標就是 1024 見方里的原始值，分類器自己歸一。
#[test]
fn classifies_straight_directions() {
    // 橫：向右微上
    assert_eq!(first_class(&[[100.0, 500.0], [700.0, 510.0]]), 'h');
    // 提：向右上，按「提歸橫」記 h
    assert_eq!(first_class(&[[300.0, 300.0], [700.0, 500.0]]), 'h');
    // 豎：近豎直向下（y 向上，向下是負）
    assert_eq!(first_class(&[[500.0, 700.0], [510.0, 200.0]]), 's');
    // 撇：向左下
    assert_eq!(first_class(&[[600.0, 800.0], [300.0, 400.0]]), 'p');
    // 平撇：淺淺地向左下，仍然是撇
    assert_eq!(first_class(&[[600.0, 650.0], [200.0, 600.0]]), 'p');
    // 捺：向右下
    assert_eq!(first_class(&[[400.0, 600.0], [700.0, 300.0]]), 'n');
}

#[test]
fn classifies_turns_short_and_degenerate() {
    // 橫折：先右後下，一處尖角
    assert_eq!(
        first_class(&[[200.0, 700.0], [600.0, 710.0], [610.0, 300.0]]),
        'z'
    );
    // 豎提：先下後右上，一處尖角
    assert_eq!(
        first_class(&[[500.0, 700.0], [520.0, 300.0], [650.0, 280.0]]),
        'z'
    );
    // 撇是漸彎的：一段只偏一點，不成尖角
    assert_eq!(
        first_class(&[
            [700.0, 800.0],
            [600.0, 700.0],
            [500.0, 620.0],
            [420.0, 560.0]
        ]),
        'p'
    );
    // 點：太短分不出走向，記 n
    assert_eq!(first_class(&[[500.0, 600.0], [515.0, 589.0]]), 'n');
    // 單點折線沒有方向
    assert_eq!(first_class(&[[500.0, 500.0]]), '?');
}
