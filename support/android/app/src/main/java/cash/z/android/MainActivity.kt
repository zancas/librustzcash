package cash.z.android

import android.os.Bundle
import android.support.design.widget.Snackbar
import android.support.v7.app.AppCompatActivity
import android.support.v7.widget.LinearLayoutManager
import android.support.v7.widget.RecyclerView
import cash.z.wallet.Wallet
import kotlinx.android.synthetic.main.activity_main.*

class MainActivity : AppCompatActivity() {
    private val mWallet = Wallet(ByteArray(32) {0}, "127.0.0.1:0")

    private lateinit var recyclerView: RecyclerView
    private lateinit var viewAdapter: AccountAdapter
    private lateinit var viewManager: RecyclerView.LayoutManager


    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
        setSupportActionBar(toolbar)

        var accountsList = ArrayList<Pair<String, String>>()

        viewManager = LinearLayoutManager(this)
        viewAdapter = AccountAdapter(accountsList)

        recyclerView = findViewById<RecyclerView>(R.id.list).apply {
            setHasFixedSize(true)
            layoutManager = viewManager
            adapter = viewAdapter
        }

        fab.setOnClickListener { view ->
            mWallet.createAccount(java.util.UUID.randomUUID().toString().substring(0..7))
            val accounts = mWallet.accounts()
            accountsList.clear()
            for (i in 0 until accounts.size) {
                accountsList.add(Pair(accounts[i], mWallet.defaultAddressForAccount(i)))
            }
            viewAdapter.notifyDataSetChanged()
            Snackbar.make(
                    view,
                    "Created account! Now you have " + accounts.size + " of them.",
                    Snackbar.LENGTH_LONG).setAction("Action", null).show()
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        mWallet.destroy()
    }

}
