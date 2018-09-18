/**
 * Copyright (C) 2018 The Zcash Developers
 */
package cash.z.wallet

class JNIWallet {
    init {
        System.loadLibrary("zcashwallet")
    }

    external fun init(seed: ByteArray, zcsd: String): Long

    external fun destroy(walletPtr: Long)

    external fun createAccount(walletPtr: Long, label: String)

    external fun accounts(walletPtr: Long): Array<String>

    external fun defaultAddressForAccount(walletPtr: Long, account: Int): String

    external fun balancesForAccount(walletPtr: Long, account: Int): Array<Int>
}