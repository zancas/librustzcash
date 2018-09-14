/**
 * Copyright (C) 2018 The Zcash Developers
 */
package cash.z.wallet

class Wallet(seed: ByteArray, zcsd: String) {
    private val mJNI = JNIWallet()
    private var mWalletPtr: Long? = mJNI.init(seed, zcsd)

    fun destroy() {
        mJNI.destroy(mWalletPtr!!)
        mWalletPtr = null
    }

    fun createAccount(label: String) = mJNI.createAccount(mWalletPtr!!, label)

    fun accounts() = mJNI.accounts(mWalletPtr!!)

    fun defaultAddressForAccount(account: Int) = mJNI.defaultAddressForAccount(mWalletPtr!!, account)
}
