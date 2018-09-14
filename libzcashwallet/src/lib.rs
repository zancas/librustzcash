extern crate jni;
extern crate zcash_wallet;

use jni::{
    objects::{JClass, JString},
    sys::{jbyteArray, jint, jlong, jobjectArray, jsize, jstring},
    JNIEnv,
};
use zcash_wallet::{address::encode_payment_address, constants, Builder, Wallet};

#[no_mangle]
pub extern "system" fn Java_cash_z_wallet_JNIWallet_init(
    env: JNIEnv,
    _class: JClass,
    seed: jbyteArray,
    zcsd: JString,
) -> jlong {
    let seed = env.convert_byte_array(seed).unwrap();
    let zcsd: String = env
        .get_string(zcsd)
        .expect("Couldn't get Java string!")
        .into();

    let wallet = Builder::new()
        .local_key_store(&seed)
        .build();

    Box::into_raw(Box::new(wallet)) as jlong
}

#[no_mangle]
pub extern "system" fn Java_cash_z_wallet_JNIWallet_destroy(
    _env: JNIEnv,
    _class: JClass,
    wallet_ptr: jlong,
) {
    let _wallet = unsafe { Box::from_raw(wallet_ptr as *mut Wallet) };
}

#[no_mangle]
pub extern "system" fn Java_cash_z_wallet_JNIWallet_createAccount(
    env: JNIEnv,
    _class: JClass,
    wallet_ptr: jlong,
    label: JString,
) {
    let wallet = unsafe { &mut *(wallet_ptr as *mut Wallet) };
    let label: String = env
        .get_string(label)
        .expect("Couldn't get Java string!")
        .into();

    wallet.create_account(label);
}

#[no_mangle]
pub extern "system" fn Java_cash_z_wallet_JNIWallet_accounts(
    env: JNIEnv,
    _class: JClass,
    wallet_ptr: jlong,
) -> jobjectArray {
    let wallet = unsafe { &mut *(wallet_ptr as *mut Wallet) };

    let labels: Vec<JString> = wallet
        .accounts()
        .iter()
        .map(|a| env.new_string(&a.label))
        .collect::<Result<_, _>>()
        .unwrap();

    let jaccounts =
        env.new_object_array(
            labels.len() as jsize,
            "java/lang/String",
            *env.new_string("").unwrap(),
        ).unwrap();
    for (i, label) in labels.into_iter().enumerate() {
        env.set_object_array_element(jaccounts, i as jsize, *label)
            .unwrap();
    }
    jaccounts
}

#[no_mangle]
pub extern "system" fn Java_cash_z_wallet_JNIWallet_defaultAddressForAccount(
    env: JNIEnv,
    _class: JClass,
    wallet_ptr: jlong,
    account: jint,
) -> jstring {
    let wallet = unsafe { &mut *(wallet_ptr as *mut Wallet) };

    let addr = wallet.accounts()[account as usize].default_address();

    let output =
        env.new_string(encode_payment_address(
            constants::HRP_SAPLING_EXTENDED_SPENDING_KEY_TEST,
            &addr,
        )).expect("Couldn't create Java string!");
    output.into_inner()
}
