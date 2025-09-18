contract;

use storage_encoded::*;

storage {
    val in 0x0000000000000000000000000000000000000000000000000000000000000000: StorageEncoded = StorageEncoded {},
}


struct Foobar {
    a: u256,
    b: u64,
    c: u8,
}

impl Contract {
    #[storage(read, write)]
    fn test_set() {
        storage.val.set::<Foobar>(Foobar { a: 42, b: 1337, c: 77 });
    }
    #[storage(read, write)]
    fn test_get() {
        let res = storage.val.get::<Foobar>();
        assert_eq(res.a, 42);
        assert_eq(res.b, 1337);
        assert_eq(res.c, 77);
    }

}

#[test]
fn test_contract() {
    let c = abi(ExampleAbi, CONTRACT_ID);
    c.test_set();
    c.test_get();
}
