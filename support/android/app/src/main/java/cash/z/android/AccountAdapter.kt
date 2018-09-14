package cash.z.android

import android.support.v7.widget.RecyclerView
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import kotlinx.android.synthetic.main.accounts_item_row.view.*

class AccountAdapter(private var myAccounts: ArrayList<Pair<String, String>>) :
        RecyclerView.Adapter<AccountAdapter.MyViewHolder>() {

    class MyViewHolder(v: View): RecyclerView.ViewHolder(v) {
        private var accountName = v.accountName
        private var saplingAddress = v.saplingAddress

        fun setName(name: String, address: String) {
            accountName.text = name
            saplingAddress.text = address
        }
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): MyViewHolder {
        val view = LayoutInflater.from(parent.context)
                .inflate(R.layout.accounts_item_row, parent, false)
        return MyViewHolder(view)
    }

    override fun onBindViewHolder(holder: MyViewHolder, position: Int) {
        holder.setName(
                "Account " + position + ": " + myAccounts[position].first,
                myAccounts[position].second)
    }

    override fun getItemCount() = myAccounts.size
}