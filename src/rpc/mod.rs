pub mod client;
pub mod types;


// bitcoin-cli -regtest -rpcuser=polaruser -rpcpassword=polarpass -rpcport=18443 getnewaddress
// bitcoin-cli -regtest -rpcuser=polaruser -rpcpassword=polarpass -rpcport=18443 generatetoaddress 3 <paste_address_here>

// Transaction
// bitcoin-cli -regtest -rpcuser=polaruser -rpcpassword=polarpass -rpcport=18443 getnewaddress
// bitcoin-cli -regtest -rpcuser=polaruser -rpcpassword=polarpass -rpcport=18443 getbalance
// bitcoin-cli -regtest -rpcuser=polaruser -rpcpassword=polarpass -rpcport=18443 scantxoutset start '["addr(addr)"]'
// bitcoin-cli -regtest -rpcuser=polaruser -rpcpassword=polarpass -rpcport=18443 sendtoaddress <address> <amount>
// bitcoin-cli -regtest -rpcuser=polaruser -rpcpassword=polarpass -rpcport=18443 sendtoaddress bcrt1qxxx... 1.5