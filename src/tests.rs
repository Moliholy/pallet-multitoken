use crate::{mock::*, Error, Event};
use frame_support::{assert_noop, assert_ok};
use frame_system::ensure_signed;

#[test]
fn test_creating_a_collection_should_work() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let owner = RuntimeOrigin::signed(1);
        let owner_account = ensure_signed(RuntimeOrigin::signed(1)).unwrap();
        assert_eq!(Multitoken::next_collection_id(), 0);
        assert_eq!(Multitoken::collections(0), None);
        assert_ok!(Multitoken::create(owner.clone()));
        System::assert_last_event(Event::CollectionCreated { owner: owner_account, id: 0 }.into());
        assert_eq!(Multitoken::next_collection_id(), 1);
        assert_eq!(Multitoken::collections(0), Some(ensure_signed(owner).unwrap()));
    });
}

#[test]
fn test_only_owner_can_mint() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        let owner = RuntimeOrigin::signed(1);
        let owner_account = ensure_signed(RuntimeOrigin::signed(1)).unwrap();
        let receiver = RuntimeOrigin::signed(2);
        let receiver_account = ensure_signed(receiver.clone()).unwrap();
        assert_eq!(Multitoken::balance_of(&receiver_account, &0), 0);
        assert_ok!(Multitoken::create(owner.clone()));
        assert_eq!(Multitoken::balance_of(&receiver_account, &0), 0);

        assert_noop!(Multitoken::mint(receiver, receiver_account, 0, 100), Error::<Test>::InvalidOwner);
        assert_ok!(Multitoken::mint(owner, receiver_account, 0, 100));
        System::assert_last_event(Event::TransferSingle {
            operator: owner_account,
            from: None,
            to: Some(receiver_account),
            id: 0,
            value: 100,
        }.into());
        assert_eq!(Multitoken::balance_of(&receiver_account.clone(), &0), 100);
    });
}