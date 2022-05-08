import React, { useState, useEffect} from 'react';
import Status from './Status';
import Controls from './Controls';
import Hand from './Hand';
import Hands from './Hands';
import { ToastContainer, toast } from 'react-toastify';
import 'react-toastify/dist/ReactToastify.css';
import { SecretNetworkClient, Wallet } from "secretjs";

declare global {
  interface Window {
    keplr:any;
  }
}

type Nullable<T> = T | null;
type PT = {
  PlayerTurn: {
    player_seat: number;
    is_first: boolean;
    turn_start_time: number;
  };
};
type GameState = string | PT;
type Card = { value: string; suit: string };
type H = { cards: Card[]; total_value: number };
type Player = { address: string; hand: Nullable<H>; state: string };
type Table = {
  players_count: number;
  players: Player[];
  dealer_hand: Nullable<H>;
  state: GameState;
};

type PlayerScore = {
  address: string;
  won: boolean;
  score: number;
  reward: string;
};
type Scores = {
  players: Nullable<PlayerScore>[];
  dealer: PlayerScore;
};

function isPlayerTurn(state: GameState): state is PT {
  return (state as PT).PlayerTurn !== undefined;
}

function hasHand(hand: Nullable<H>): hand is H {
  return (JSON.stringify(hand) !== "null");
}

function hasPlayerScore(score: Nullable<PlayerScore>): score is PlayerScore {
  return (JSON.stringify(score) !== "null");
}

function winNotification(score: number, dealerScore: number, award: number) {
  toast.success(`Wow! you won ${award} u𝕊 with the score of ${score} against ${dealerScore}`, {
    autoClose: 7000,
    hideProgressBar: false,
    closeOnClick: true,
    pauseOnHover: true,
    draggable: false,
    progress: undefined,
    });
}

function lostNotification(score: number, dealerScore: number, bid: number) {
  toast.error(`Oh well.... you lost ${bid} u𝕊 with the score of ${score} against ${dealerScore}`, {
    autoClose: 7000,
    hideProgressBar: false,
    closeOnClick: true,
    pauseOnHover: true,
    draggable: false,
    progress: undefined,
    });
}

function loading() {
  toast.info("Loading...", {
    autoClose: 3000,
    hideProgressBar: false,
    closeOnClick: true,
    pauseOnHover: true,
    draggable: false,
    progress: undefined,
    });
}

const App: React.FC = () => {
  enum Message {
    sit = 'Take a sit',
    dealerTurn = 'Now it is my turn!',
    gameSit = 'Game is on, take a sit',
    bet = 'Place a Bet!',
    hitHold = 'Hit or Hold?',
    bust = 'Bust!',
    blackjack = 'It is your lucky day!',
    beReady = 'Be ready, you are playing soon!',
    nextRound = 'Game is on, wait for the next round',
    userWin = 'You Win!',
    tie = 'Tie!',

  }

  const [address, setAddress] = useState("");

  const [table, setTable] = useState<Table>({
    players_count: 0,
    players: [{
      address: '',
      hand: {
        cards: [],
        total_value: 0
      },
      state: ''
    },{
      address: '',
      hand: {
        cards: [],
        total_value: 0
      },
      state: ''
    },{
      address: '',
      hand: {
        cards: [],
        total_value: 0
      },
      state: ''
    },{
      address: '',
      hand: {
        cards: [],
        total_value: 0
      },
      state: ''
    },{
      address: '',
      hand: {
        cards: [],
        total_value: 0
      },
      state: ''
    },{
      address: '',
      hand: {
        cards: [],
        total_value: 0
      },
      state: ''
    },],
    dealer_hand: null,
    state: ''
  });

  const [balance, setBalance] = useState(0);

  const [controlsState, setControlsState] = useState({
  message: "", 
  buttonState: {
    hitDisabled: false,
    holdDisabled: false,
    standDisabled: false,
  },
  kickButtonsState: [{canBeKicked: false, kickTimer:100},{canBeKicked: true, kickTimer:0},{canBeKicked: false, kickTimer:100},{canBeKicked: false, kickTimer:100},{canBeKicked: false, kickTimer:100},{canBeKicked: false, kickTimer:100}],
  sitButtonsState: [{disabled:false},{disabled:true},{disabled:false},{disabled:false},{disabled:false},{disabled:false}]
  });

  const [client, setClient] = useState<SecretNetworkClient>();
  const [onlyOnce, setOnlyOnce] = useState(true);
  const [onHold, setOnHold] = useState(false);
  const gameCodeHash : string = "93B8DF51E4BF301816A00AA58672A637F9207C0EF983B261BE2AAB5F3798C647";
  const gameAddress : string = "secret1z6730hjd3ayhngu6et05nh26cccgcf69r4lh2z";

  const getClient = async () => {
    await window.keplr.enable('pulsar-2');
    const keplrOfflineSigner = window.keplr.getOfflineSignerOnlyAmino('pulsar-2');
    const [{ address: keplrAddress }] = await keplrOfflineSigner.getAccounts();
    const walletPubAddress = keplrAddress;

    setAddress(walletPubAddress);
    const nodeRpcAddress : string = "https://lior.node.scrtlabs.com";
    const client = await SecretNetworkClient.create({
      grpcWebUrl: nodeRpcAddress,
      chainId: 'pulsar-2',
      wallet: keplrOfflineSigner,
      walletAddress: walletPubAddress,
    });

    // const address = "secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne";
    // const mnemonic = "jelly shadow frog dirt dragon use armed praise universe win jungle close inmate rain oil canvas beauty pioneer chef soccer icon dizzy thunder meadow";
    // const wallet = new Wallet(mnemonic);
    // setAddress(address);
    // const nodeRpcAddress : string = "http://localhost:9091";
    // const client = await SecretNetworkClient.create({
    //   grpcWebUrl: nodeRpcAddress,
    //   chainId: 'secretdev-1',
    //   wallet: wallet,
    //   walletAddress: address,
    // });
  
    return client;
  }

  function toArrayBuffer(buf: Uint8Array): ArrayBuffer {
    const ab = new ArrayBuffer(buf.length);
    const view = new Uint8Array(ab);
    for (let i = 0; i < buf.length; ++i) {
      view[i] = buf[i];
    }
    return ab;
  }
  
  const getTable = async (
    client: SecretNetworkClient,
    gameCodeHash: string,
    gameAddress: string
  ): Promise<Table> => {
    type TableResponse = { get_table: { table: Uint8Array } };

    const tableResponse = (await client.query.compute.queryContract({
      contractAddress: gameAddress,
      codeHash: gameCodeHash,
      query: { get_table: {} },
    })) as TableResponse;

    return JSON.parse(
      new TextDecoder().decode(toArrayBuffer(tableResponse.get_table.table))
    );
  }

  async function getScores(
    client: SecretNetworkClient,
    gameCodeHash: string,
    gameAddress: string
  ): Promise<Scores> {
    type ScoresResponse = { get_last_score: { last_score: Uint8Array } };
  
    const scoresResponse = (await client.query.compute.queryContract({
      contractAddress: gameAddress,
      codeHash: gameCodeHash,
      query: { get_last_score: {} },
    })) as ScoresResponse;
  
    return JSON.parse(
      new TextDecoder().decode(
        toArrayBuffer(scoresResponse.get_last_score.last_score)
      )
    );
  }

  const getUserBalance = async (
    client: SecretNetworkClient,
    gameCodeHash: string,
    gameAddress: string
  ): Promise<number> => {
    type UserBalanceResponse = { get_user_balance: { balance: string } };

    const userBalanceResponse = (await client.query.compute.queryContract({
      contractAddress: gameAddress,
      codeHash: gameCodeHash,
      query: { get_user_balance: { address: address } },
    })) as UserBalanceResponse;

    return parseInt(userBalanceResponse.get_user_balance.balance);
  }

  const refreshTableState = async () => {
    const newTable = await getTable(client!, gameCodeHash, gameAddress);

    setTable(newTable);
    setBalance(await getUserBalance(client!, gameCodeHash, gameAddress));
  }

  useEffect(() => {
    async function createClient() {
      await window.keplr.experimentalSuggestChain({
        chainId: "pulsar-2",
        chainName: "Pulsar",
        rpc: "http://40.88.137.151:26657",
        rest: "http://40.88.137.151:1317",
        bip44: {
          coinType: 529,
        },
        bech32Config: {
          bech32PrefixAccAddr: "secret",
          bech32PrefixAccPub: "secretpub",
          bech32PrefixValAddr: "secretvaloper",
          bech32PrefixValPub: "secretvaloperpub",
          bech32PrefixConsAddr: "secretvalcons",
          bech32PrefixConsPub: "secretvalconspub",
        },
        currencies: [
          {
            coinDenom: "SCRT",
            coinMinimalDenom: "uscrt",
            coinDecimals: 6,
            coinGeckoId: "secret",
          },
        ],
        feeCurrencies: [
          {
            coinDenom: "SCRT",
            coinMinimalDenom: "uscrt",
            coinDecimals: 6,
            coinGeckoId: "secret",
          },
        ],
        stakeCurrency: {
          coinDenom: "SCRT",
          coinMinimalDenom: "uscrt",
          coinDecimals: 6,
          coinGeckoId: "secret",
        },
        coinType: 529,
        gasPriceStep: {
          low: 0.1,
          average: 0.25,
          high: 1,
        },
        features: ["secretwasm", "stargate", "ibc-transfer", "ibc-go"],
      });

      setClient(await getClient());
    }

    createClient();
  }, []);

  useEffect(() => {
    const isAddressOk = () => {
      if(typeof client === "undefined") {
        return false;
      }

      return (client!.address === address);
    }

    if(onlyOnce && isAddressOk()) {
      setInterval(refreshTableState, 1000);
      setOnlyOnce(false);
    }

  }, [JSON.stringify(client)]);

  

  useEffect(() => {
    console.info(JSON.stringify(table));
    let newSitButtonsState = controlsState.sitButtonsState;
    let newKickButtonsState = controlsState.kickButtonsState;
    let newMessage = controlsState.message;
    let newButtonState = {hitDisabled: false, holdDisabled: false, standDisabled: false};

    const updatePlayerButtons = () => {
      for (let i:number = 0; i < 6; i++) {
        if (table.players[i].address === '') {
          newSitButtonsState[i].disabled = (findMySeat(table) !== -1);
          newKickButtonsState[i].canBeKicked = false;
          newKickButtonsState[i].kickTimer = 0;
        } else {
          newSitButtonsState[i].disabled = true;
          if (isPlayerTurn(table.state)) {
            const pt = (table.state as PT).PlayerTurn;
            if(pt.player_seat !== i) {
              newKickButtonsState[i].canBeKicked = false;
              newKickButtonsState[i].kickTimer = 0;
            } else {
              
              newKickButtonsState[i].canBeKicked = (findMySeat(table) !== i);
              let timer = 300 - (Math.round(Date.now() / 1000) - pt.turn_start_time);
              if(timer < 0) {
                timer = 0;
              }

              newKickButtonsState[i].kickTimer = timer;
            }
          } else {
            newKickButtonsState[i].canBeKicked = false;
            newKickButtonsState[i].kickTimer = 0;
          }
        }      
      }
    }
    
    const parseTableState = () => {
      if(!isPlayerTurn(table.state)) {
        if (table.state === "NoPlayers") {
          newMessage = Message.sit;
          return;
        } 
        
        if(table.state === "DealerTurn") {
          newMessage = Message.dealerTurn;
          return;
        }

      } 
      
      const pt = (table.state as PT).PlayerTurn;
      const seat = findMySeat(table);

      if(seat === -1) {
        newMessage = Message.gameSit;
        return;
      }
      
      const ps = table.players[seat].state;
      switch(ps) {
        case 'Bid':
        case 'Hit':
          newMessage = Message.hitHold;
          newButtonState = {hitDisabled: false, holdDisabled: false, standDisabled: true};
          break;
        case 'Hold':
          newButtonState = {hitDisabled: true, holdDisabled: true, standDisabled: true};
          break;
      }
        
      let playerScore = 0;
      if(hasHand(table.players[seat].hand)) {
        playerScore = getPlayerScore(seat);
      }
        
      if(pt.player_seat !== seat) {
        if (seat > pt.player_seat) {
          newMessage = Message.beReady;
          return;
        } 
        
        if (playerScore === 0) {
          newMessage = Message.nextRound;
            return;
        }

        return;
      }
      
      if (pt.is_first) {
          newMessage = Message.bet;
          return;
      } 
      
      if(!onHold) {
        if(playerScore === 21) {
            newMessage = Message.blackjack;
            newButtonState = {hitDisabled: true, holdDisabled: true, standDisabled: true};
            hold();
            return;
        }
        
        if(playerScore > 21) {
            newMessage = Message.bust;
            newButtonState = {hitDisabled: true, holdDisabled: true, standDisabled: true};
            hold();
            return;
        }
      }
    }

    updatePlayerButtons();
    parseTableState();
    setControlsState({
      sitButtonsState: newSitButtonsState,
      kickButtonsState: newKickButtonsState,
      message: newMessage,
      buttonState: newButtonState
    });
    
  }, [JSON.stringify(table)]);

  const placeBet = async (amount: number) => {
    const seat = findMySeat(table);
    if(seat === -1) {
      alert("First take a seat");
    }

    loading();

    const tx = await client!.tx.compute.executeContract(
      {
        sender: address,
        contractAddress: gameAddress,
        codeHash: gameCodeHash,
        msg: {
          bid: {
            amount: amount.toString(),
            seat: seat,
          },
        },
        sentFunds: [{ amount: amount.toString(), denom: "uscrt" }],
      },
      {
        gasLimit: 80000,
      }
    );
    
    if(tx.code !== 0) {
      console.warn(tx.rawLog);
    }
  }

  const hit = async () => {
    const seat = findMySeat(table);
    if(seat === -1) {
      alert("First take a seat");
    }

    loading();

    const tx = await client!.tx.compute.executeContract(
      {
        sender: address,
        contractAddress: gameAddress,
        codeHash: gameCodeHash,
        msg: {
          hit: {
            seat: seat,
          },
        },
        sentFunds: [],
      },
      {
        gasLimit: 100000,
      }
    );

    if(tx.code !== 0) {
      console.warn(tx.rawLog);
    }
  }

  const hold = async () => {
    const seat = findMySeat(table);
    if(seat === -1) {
      alert("First take a seat");
    }

    loading();
    setOnHold(true);

    const tx = await client!.tx.compute.executeContract(
      {
        sender: address,
        contractAddress: gameAddress,
        codeHash: gameCodeHash,
        msg: {
          hold: {
            seat: seat,
          },
        },
        sentFunds: [],
      },
      {
        gasLimit: 300000,
      }
    );

    setOnHold(false);

    if(tx.code !== 0) {
      console.warn(tx.rawLog);
    }
  }

  const findMySeat = (t : Table) => {
    if(typeof client === "undefined") {
      return -1;
    }

    for (let i: number = 0; i < 6; i++) {
      if(t.players[i].address === client!.address) {
        return i;
      }
    }

    return -1;
  }

  const stand = async () => {
    const seat = findMySeat(table);
    if (seat === -1) {
      alert("Player isn't seated");
    }


    loading();

    const tx = await client!.tx.compute.executeContract(
      {
        sender: client!.address,
        contractAddress: gameAddress,
        codeHash: gameCodeHash,
        msg: {
          stand: {
            seat: seat,
          },
        },
        sentFunds: [],
      },
      {
        gasLimit: 100000,
      }
    );

    if(tx.code !== 0) {
      console.warn(tx.rawLog);
    }
  }

  const getLastScore = async () => {
    const seat = findMySeat(table);
    if(seat !== -1) {
      const scores = await getScores(client!, gameCodeHash, gameAddress);
      if(hasPlayerScore(scores.players[seat])) {
        const playerScore = scores.players[seat]!;
        if(playerScore.address === client!.address) {
          if(playerScore.won) {
            winNotification(playerScore.score, scores.dealer.score, parseInt(playerScore.reward));
          } else {
            lostNotification(playerScore.score, scores.dealer.score, parseInt(playerScore.reward));
          }
        }
      }
    }
  }

  const kickEvent = async (index: number) => {
    const kickedAddress = table.players[index].address;
    loading();

    const tx = await client!.tx.compute.executeContract(
      {
        sender: client!.address,
        contractAddress: gameAddress,
        codeHash: gameCodeHash,
        msg: {
          kick: {
            target: kickedAddress,
            seat: index,
          },
        },
        sentFunds: [],
      },
      {
        gasLimit: 100000,
      }
    );

    if(tx.code !== 0) {
      console.warn(tx.rawLog);
    }
  }

  const sitEvent = async (index: number) => {
    loading();
    const tx = await client!.tx.compute.executeContract(
      {
        sender: client!.address,
        contractAddress: gameAddress,
        codeHash: gameCodeHash,
        msg: {
          sit: {
            secret: Math.floor(Math.random()*(9223372036854775807-1+1)+1),
            seat: index,
          },
        },
        sentFunds: [],
      },
      {
        gasLimit: 100000,
      }
    );

    if(tx.code !== 0) {
      console.warn(tx.rawLog);
    }
  }

  const toGameCards = (cards: Card[]) => {
    for (let card of cards) {
      switch(card.value) {
        case 'Two':
          card.value = "2";
          break;
        case 'Three':
          card.value = "3";
          break;
        case 'Four':
          card.value = "4";
          break;
        case 'Five':
          card.value = "5";
          break;
        case 'Six':
          card.value = "6";
          break;
        case 'Seven':
          card.value = "7";
          break;
        case 'Eight':
          card.value = "8";
          break;
        case 'Nine':
          card.value = "9";
          break;
        case 'Ten':
          card.value = "10";
          break;
        case 'Jack':
          card.value = "J";
          break;
        case 'Queen':
          card.value = "Q";
          break;
        case 'King':
          card.value = "K";
          break;
        case 'Ace':
          card.value = "A";
          break;
      }
      switch(card.suit) {
        case 'Spade':
          card.suit = '♠';
          break;
        case 'Diamond':
          card.suit = '♦';
          break;
        case 'Club':
          card.suit = '♣';
          break;
        case 'Heart':
          card.suit = '♥';
      }
    }

    return cards;
  }

  const cardToValue = (card: Card) => {
    switch(card.value) {
      case '2':
      case '3':
      case '4':
      case '5':
      case '6':
      case '7':
      case '8':
      case '9':
      case '10':
        return parseInt(card.value);
      case 'J':
      case 'Q':
      case 'K':
        return 10;
      case 'A':  
        return 1;
      default:
        return 0;
    }
  }

  const getPlayerCards = (index: number) => {
    if(!hasHand(table.players[index].hand)) {
      return [];
    }

    return toGameCards(table.players[index].hand!.cards);
  }

  const getPlayerScore = (index: number) => {
    return getHandScore(table.players[index].hand);
  }

  const getDealerScore = () => {
    return getHandScore(table.dealer_hand);
  }

  const getHandScore = (hand: Nullable<H>) => {
    if(!hasHand(hand)) {
      return 0;
    }

    let sum = 0;
    let hadAce = false;
    let gameCards = toGameCards(hand!.cards);
    for(const card of gameCards) {
      sum += cardToValue(card);
      if(card.value === "A") {
        hadAce = true;
      }
    }

    if(sum <= 11 && hadAce) {
      sum+= 10;
    }

    return sum;
  }

  const getDealerCards = () => {
    if(!hasHand(table.dealer_hand)) {
      return [];
    }

    return toGameCards(table.dealer_hand!.cards);
  }

  const isPlayerRound = () => {
    const seat = findMySeat(table);
    if(seat === -1) {
      return false;
    }

    if(!isPlayerTurn(table.state)) {
      return false;
    }

    const pt = (table.state as PT).PlayerTurn;
    return (pt.player_seat === seat);
  }

  const isPlayerFirstRound = () => {
    const pt = (table.state as PT).PlayerTurn;
    return isPlayerRound() && pt.is_first;
  }

  return (
    <>
      <Status message={controlsState.message} balance={balance} address={address}/>
      <Controls
        buttonState={controlsState.buttonState}
        isFirstRound={isPlayerFirstRound()}
        isMyRound={isPlayerRound()}
        isSeated={findMySeat(table) !== -1}
        betEvent={placeBet}
        hitEvent={hit}
        holdEvent={hold}
        standEvent={stand}
        lastScoreEvent={getLastScore}
      />
      <Hand title={`Dealer's Hand`} cards={getDealerCards()} isDealer={true} dealerScore={getDealerScore()} />
      <Hands addresses={[table.players[0].address,
      table.players[1].address,
      table.players[2].address,
      table.players[3].address,
      table.players[4].address,
      table.players[5].address,
      ]}
      cardsArr= {[getPlayerCards(0), 
        getPlayerCards(1),
        getPlayerCards(2),
        getPlayerCards(3),
        getPlayerCards(4),
        getPlayerCards(5)]} 
      scores= {[getPlayerScore(0),
        getPlayerScore(1),
        getPlayerScore(2),
        getPlayerScore(3),
        getPlayerScore(4),
        getPlayerScore(5),
      ]

      }
      kickButtonsState={controlsState.kickButtonsState}
      sitButtonsState={controlsState.sitButtonsState}
      kickEvent={kickEvent}
      sitEvent={sitEvent}
      />
      <ToastContainer position="bottom-right"/>
    </>
  );
}

export default App;
