use ra_net::SequenceWindow;

#[test]
fn accepts_in_order_only() {
    let mut w = SequenceWindow::default();
    assert!(w.accept(0));
    assert!(!w.accept(0));
    assert!(!w.accept(2));
    assert!(w.accept(1));
    assert_eq!(w.next, 2);
}
