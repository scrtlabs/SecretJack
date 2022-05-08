import axios from "axios";
import { Wallet, SecretNetworkClient, fromUtf8 } from "secretjs";
import fs from "fs";
import assert from "assert";

// Returns a client with which we can interact with secret network
const initializeClient = async (endpoint: string, chainId: string) => {
  const wallet = new Wallet(); // Use default constructor of wallet to generate random mnemonic.
  const accAddress = wallet.address;
  const client = await SecretNetworkClient.create({
    // Create a client to interact with the network
    grpcWebUrl: endpoint,
    chainId: chainId,
    wallet: wallet,
    walletAddress: accAddress,
  });

  console.log(`Initialized client with wallet address: ${accAddress}`);
  return client;
};

const uploadContract = async (
  client: SecretNetworkClient,
  contractPath: string,
  contractName: string
) => {
  const wasmCode = fs.readFileSync(contractPath);
  console.log(`Uploading ${contractName} contract`);

  const uploadReceipt = await client.tx.compute.storeCode(
    {
      wasmByteCode: wasmCode,
      sender: client.address,
      source: "",
      builder: "",
    },
    {
      gasLimit: 5000000,
    }
  );

  if (uploadReceipt.code !== 0) {
    console.log(
      `Failed to get code id: ${JSON.stringify(uploadReceipt.rawLog)}`
    );
    throw new Error(`Failed to upload contract`);
  }

  const codeIdKv = uploadReceipt.jsonLog![0].events[0].attributes.find(
    (a: any) => {
      return a.key === "code_id";
    }
  );

  const codeId = Number(codeIdKv!.value);
  console.log(`${contractName} Game contract codeId: ${codeId}`);

  const codeHash = await client.query.compute.codeHash(codeId);
  console.log(`${contractName} contract hash: ${codeHash}`);

  var contractInfo: [number, string] = [codeId, codeHash];

  return contractInfo;
};

// Stores and instantiates a new contract in our network
const initializeContracts = async (
  client: SecretNetworkClient,
  gameContractPath: string,
  bankContractPath: string
) => {
  const [gameCodeId, gameCodeHash] = await uploadContract(
    client,
    gameContractPath,
    "Game"
  );
  const [bankCodeId, bankCodeHash] = await uploadContract(
    client,
    bankContractPath,
    "Bank"
  );

  const contract = await client.tx.compute.instantiateContract(
    {
      sender: client.address,
      codeId: bankCodeId,
      initMsg: {
        game_contract_code_id: gameCodeId,
        game_contract_code_hash: gameCodeHash,
        secret: 1234,
      },
      codeHash: bankCodeHash,
      label: "Bank" + Math.ceil(Math.random() * 10000), // The label should be unique for every contract, add random string in order to maintain uniqueness
    },
    {
      gasLimit: 4000000,
    }
  );

  if (contract.code !== 0) {
    throw new Error(
      `Failed to instantiate the contract with the following error ${contract.rawLog}`
    );
  }

  const gameContractAddress = (
    await client.query.compute.contractsByCode(gameCodeId)
  ).contractInfos[0].address;
  const bankContractAddress = (
    await client.query.compute.contractsByCode(bankCodeId)
  ).contractInfos[0].address;

  console.log(`Game contract address: ${gameContractAddress}`);
  console.log(`Bank contract address: ${bankContractAddress}`);

  var contractInfo: [[string, string], [string, string]] = [
    [gameCodeHash, gameContractAddress],
    [bankCodeHash, bankContractAddress],
  ];
  return contractInfo;
};

const getFromFaucet = async (address: string) => {
  await axios.get(`http://localhost:5000/faucet?address=${address}`);
};

async function getScrtBalance(userCli: SecretNetworkClient): Promise<string> {
  let balanceResponse = await userCli.query.bank.balance({
    address: userCli.address,
    denom: "uscrt",
  });
  return balanceResponse.balance!.amount;
}

async function fillUpFromFaucet(
  client: SecretNetworkClient,
  targetBalance: Number
) {
  let balance = await getScrtBalance(client);
  while (Number(balance) < targetBalance) {
    try {
      await getFromFaucet(client.address);
    } catch (e) {}
    balance = await getScrtBalance(client);
  }
  console.error(`got tokens from faucet: ${balance}`);
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
type Hand = { cards: Card[]; total_value: number };
type Player = { address: string; hand: Nullable<Hand>; state: string };
type Table = {
  players_count: number;
  players: Player[];
  dealer_hand: Nullable<Hand>;
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

// Initialization procedure
async function initializeAndUploadContracts() {
  let endpoint = "http://localhost:9091";
  let chainId = "secretdev-1";

  const client = await initializeClient(endpoint, chainId);

  await fillUpFromFaucet(client, 1_000_000_000);

  const [[gameCodeHash, gameAddress], [bankCodeHash, bankAddress]] =
    await initializeContracts(client, "game.wasm", "bank.wasm");

  const sendFunds = {
    fromAddress: client.address,
    toAddress: bankAddress,
    amount: [{ denom: "uscrt", amount: "250000000" }],
  };

  const tx = await client.tx.bank.send(sendFunds, {
    gasLimit: 100000,
  });

  if (tx.code != 0) {
    throw new Error("Failed sending funds to bank contract");
  }

  var clientInfo: [SecretNetworkClient, [string, string], [string, string]] = [
    client,
    [gameCodeHash, gameAddress],
    [bankCodeHash, bankAddress],
  ];
  return clientInfo;
}

async function queryBankBalance(
  client: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string
): Promise<string> {
  type BankBalanceResponse = { get_bank_balance: { balance: string } };

  const bankBalanceResponse = (await client.query.compute.queryContract({
    contractAddress: bankAddress,
    codeHash: bankCodeHash,
    query: { get_bank_balance: {} },
  })) as BankBalanceResponse;

  if ('err"' in bankBalanceResponse) {
    throw new Error(
      `Query failed with the following err: ${JSON.stringify(
        bankBalanceResponse
      )}`
    );
  }

  return bankBalanceResponse.get_bank_balance.balance;
}

async function withdrawAllBankFunds(
  client: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string
) {
  const tx = await client.tx.compute.executeContract(
    {
      sender: client.address,
      contractAddress: bankAddress,
      codeHash: bankCodeHash,
      msg: {
        emergency_withdraw_all_funds: {},
      },
      sentFunds: [],
    },
    {
      gasLimit: 100000,
    }
  );

  //let parsedTransactionData = JSON.parse(fromUtf8(tx.data[0])); // In our case we don't really need to access transaction data
  console.log(`withdraw all used ${tx.gasUsed} gas`);
}

async function payToWinner(
  client: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string
) {
  const tx = await client.tx.compute.executeContract(
    {
      sender: client.address,
      contractAddress: bankAddress,
      codeHash: bankCodeHash,
      msg: {
        pay_to_winner: {
          amount: "1000000",
          to: client.address,
        },
      },
      sentFunds: [],
    },
    {
      gasLimit: 100000,
    }
  );

  //let parsedTransactionData = JSON.parse(fromUtf8(tx.data[0])); // In our case we don't really need to access transaction data
  console.log(`pay to winner used ${tx.gasUsed} gas`);
}

async function queryUserBalance(
  client: SecretNetworkClient,
  gameCodeHash: string,
  gameAddress: string,
  address: string
): Promise<string> {
  type UserBalanceResponse = { get_user_balance: { balance: string } };

  const userBalanceResponse = (await client.query.compute.queryContract({
    contractAddress: gameAddress,
    codeHash: gameCodeHash,
    query: { get_user_balance: { address: address } },
  })) as UserBalanceResponse;

  if ('err"' in userBalanceResponse) {
    throw new Error(
      `Query failed with the following err: ${JSON.stringify(
        userBalanceResponse
      )}`
    );
  }

  return userBalanceResponse.get_user_balance.balance;
}

async function getTable(
  client: SecretNetworkClient,
  gameCodeHash: string,
  gameAddress: string
): Promise<Table> {
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

async function sit(
  client: SecretNetworkClient,
  gameCodeHash: string,
  gameAddress: string,
  seat: number
) {
  const tx = await client.tx.compute.executeContract(
    {
      sender: client.address,
      contractAddress: gameAddress,
      codeHash: gameCodeHash,
      msg: {
        sit: {
          secret: 1234,
          seat: seat,
        },
      },
      sentFunds: [],
    },
    {
      gasLimit: 100000,
    }
  );

  //let parsedTransactionData = JSON.parse(fromUtf8(tx.data[0])); // In our case we don't really need to access transaction data
  console.log(`sit used ${tx.gasUsed} gas`);
}

async function bid(
  client: SecretNetworkClient,
  gameCodeHash: string,
  gameAddress: string,
  seat: number,
  amount: string
) {
  const tx = await client.tx.compute.executeContract(
    {
      sender: client.address,
      contractAddress: gameAddress,
      codeHash: gameCodeHash,
      msg: {
        bid: {
          amount: amount,
          seat: seat,
        },
      },
      sentFunds: [{ amount: amount, denom: "uscrt" }],
    },
    {
      gasLimit: 80000,
    }
  );

  console.log(`bid used ${tx.gasUsed} gas`);
}

async function hit(
  client: SecretNetworkClient,
  gameCodeHash: string,
  gameAddress: string,
  seat: number
) {
  const tx = await client.tx.compute.executeContract(
    {
      sender: client.address,
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

  console.log(`hit used ${tx.gasUsed} gas`);
}

async function hold(
  client: SecretNetworkClient,
  gameCodeHash: string,
  gameAddress: string,
  seat: number
) {
  const tx = await client.tx.compute.executeContract(
    {
      sender: client.address,
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

  //let parsedTransactionData = JSON.parse(fromUtf8(tx.data[0])); // In our case we don't really need to access transaction data
  console.log(`hold used ${tx.gasUsed} gas`);
}

async function stand(
  client: SecretNetworkClient,
  gameCodeHash: string,
  gameAddress: string,
  seat: number
) {
  const tx = await client.tx.compute.executeContract(
    {
      sender: client.address,
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

  //let parsedTransactionData = JSON.parse(fromUtf8(tx.data[0])); // In our case we don't really need to access transaction data
  console.log(`stand used ${tx.gasUsed} gas`);
}

async function kick(
  client: SecretNetworkClient,
  gameCodeHash: string,
  gameAddress: string,
  seat: number,
  address: string
) {
  const tx = await client.tx.compute.executeContract(
    {
      sender: client.address,
      contractAddress: gameAddress,
      codeHash: gameCodeHash,
      msg: {
        kick: {
          target: address,
          seat: seat,
        },
      },
      sentFunds: [],
    },
    {
      gasLimit: 100000,
    }
  );

  //let parsedTransactionData = JSON.parse(fromUtf8(tx.data[0])); // In our case we don't really need to access transaction data
  console.log(`kick used ${tx.gasUsed} gas`);
}

function toArrayBuffer(buf: Uint8Array): ArrayBuffer {
  const ab = new ArrayBuffer(buf.length);
  const view = new Uint8Array(ab);
  for (let i = 0; i < buf.length; ++i) {
    view[i] = buf[i];
  }
  return ab;
}

// The following functions are only some examples of how to write integration tests, there are many tests that we might want to write here.
async function test_bank_balance_on_initialization(
  client: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string,
  _gameCodeHash: string,
  _gameAddress: string
) {
  const onInitializationBalance: string = await queryBankBalance(
    client,
    bankCodeHash,
    bankAddress
  );
  assert(
    onInitializationBalance === "225000000",
    `The balance on initialization expected to be 225,000,000 instead of ${onInitializationBalance}`
  );
}

async function test_bad_bid(
  client: SecretNetworkClient,
  _bankCodeHash: string,
  _bankAddress: string,
  gameCodeHash: string,
  gameAddress: string
) {
  const seat: number = 2;
  const bet: string = "50000000";

  await sit(client, gameCodeHash, gameAddress, seat);
  await bid(client, gameCodeHash, gameAddress, seat, bet);
  await stand(client, gameCodeHash, gameAddress, seat);

  const userBalance: String = await queryUserBalance(
    client,
    gameCodeHash,
    gameAddress,
    client.address
  );
  assert(
    userBalance === "0",
    `The user shouldn't deposit more than the limit of the bank ${userBalance}`
  );
}

function isPlayerTurn(state: GameState): state is PT {
  return (state as PT).PlayerTurn !== undefined;
}

function isPlayerScore(score: Nullable<PlayerScore>): score is PlayerScore {
  return JSON.stringify(score) !== "null";
}

function getHandScore(scores: Scores, seat: number) {
  assert(isPlayerScore(scores.players[seat]), "Player has no score");

  return (scores.players[seat] as PlayerScore).score;
}

function isTheDealerWon(scores: Scores, seat: number) {
  assert(isPlayerScore(scores.players[seat]), "Player has no score");

  return !(scores.players[seat] as PlayerScore).won;
}

async function roundup(
  client: SecretNetworkClient,
  prevBankBalance: string,
  players: { seat: number; prevBalance: string }[],
  bankCodeHash: string,
  bankAddress: string,
  gameCodeHash: string,
  gameAddress: string
) {
  const bet: string = "10000000";
  const table = await getTable(client, gameCodeHash, gameAddress);
  let awards: number[] = [];
  let index = 0;

  const scores = await getScores(client, gameCodeHash, gameAddress);

  for (let player of players) {
    console.log(
      `-------------------------- Seat ${player.seat} -----------------------------`
    );
    const newUserBalance: string = await queryUserBalance(
      client,
      gameCodeHash,
      gameAddress,
      table.players[player.seat].address
    );
    assert(
      newUserBalance === "0",
      `After roundup player ${
        player.seat
      } balance should be 0 balance is ${newUserBalance} 
      ${JSON.stringify(table)}`
    );

    if (isTheDealerWon(scores, player.seat)) {
      console.log(
        `The winner is the dealer with ${getHandScore(
          scores,
          player.seat
        )} against ${scores.dealer.score}`
      );

      awards[index] = -1 * parseInt(bet);
    } else {
      console.log(
        `The winner is the player with ${getHandScore(
          scores,
          player.seat
        )} against ${scores.dealer.score}`
      );

      awards[index] = parseInt(bet);
      if (getHandScore(scores, player.seat) === 21) {
        awards[index] *= 1.25;
      }
    }

    index++;
  }

  let totalBankDiff = 0;
  for (let index: number = 0; index < players.length; index++) {
    totalBankDiff -= awards[index] * 0.9;

    if (awards[index] > 0) {
      const walletBalance: string = (
        await client.query.bank.balance({
          address: table.players[players[index].seat].address,
          denom: "uscrt",
        })
      ).balance!.amount;

      assert(
        parseInt(walletBalance) > parseInt(players[index].prevBalance),
        `Wallet balance is: ${parseInt(
          walletBalance
        )} it should be greater than prev wallet ballance ${parseInt(
          players[index].prevBalance
        )}`
      );
    }
  }

  const bankBalance: string = await queryBankBalance(
    client,
    bankCodeHash,
    bankAddress
  );

  assert(
    parseInt(bankBalance) === parseInt(prevBankBalance) + totalBankDiff,
    `Expected bank balance to be equal to ${
      parseInt(prevBankBalance) + totalBankDiff
    } actual bank balance is ${parseInt(bankBalance)}`
  );
}

async function test_game_flow(
  client: SecretNetworkClient,
  seat: number,
  _bankCodeHash: string,
  _bankAddress: string,
  gameCodeHash: string,
  gameAddress: string
) {
  const bet: string = "10000000";
  await bid(client, gameCodeHash, gameAddress, seat, bet);

  const userBalance: string = await queryUserBalance(
    client,
    gameCodeHash,
    gameAddress,
    client.address
  );
  assert(
    userBalance === bet,
    `User bid wasn't accepted during game flow ${userBalance}`
  );

  let table: Table = await getTable(client, gameCodeHash, gameAddress);
  assert(
    isPlayerTurn(table.state) && table.players[seat].state === "Bid",
    `Expected game state is player ${seat} turn`
  );

  while (table.players[seat].hand!.total_value < 17) {
    await hit(client, gameCodeHash, gameAddress, seat);
    table = await getTable(client, gameCodeHash, gameAddress);
  }

  await hold(client, gameCodeHash, gameAddress, seat);
}

async function test_normal_game_flow(
  client: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string,
  gameCodeHash: string,
  gameAddress: string
) {
  const seat: number = 3;
  await sit(client, gameCodeHash, gameAddress, seat);

  for (let i: number = 0; i < 3; i++) {
    const prevBankBalance: string = await queryBankBalance(
      client,
      bankCodeHash,
      bankAddress
    );
    const prevWalletBalance: string = (
      await client.query.bank.balance({
        address: client.address,
        denom: "uscrt",
      })
    ).balance!.amount;

    await test_game_flow(
      client,
      seat,
      bankCodeHash,
      bankAddress,
      gameCodeHash,
      gameAddress
    );

    await roundup(
      client,
      prevBankBalance,
      [{ seat: seat, prevBalance: prevWalletBalance }],
      bankCodeHash,
      bankAddress,
      gameCodeHash,
      gameAddress
    );
  }

  await stand(client, gameCodeHash, gameAddress, seat);
}

async function test_normal_game_flow_two_players(
  client: SecretNetworkClient,
  client2: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string,
  gameCodeHash: string,
  gameAddress: string
) {
  const seat: number = 3;
  const seat2: number = 5;

  await sit(client, gameCodeHash, gameAddress, seat);
  await sit(client2, gameCodeHash, gameAddress, seat2);

  for (let i: number = 0; i < 3; i++) {
    const prevBankBalance: string = await queryBankBalance(
      client,
      bankCodeHash,
      bankAddress
    );

    const prevWalletBalanceP1: string = (
      await client.query.bank.balance({
        address: client.address,
        denom: "uscrt",
      })
    ).balance!.amount;

    const prevWalletBalanceP2: string = (
      await client2.query.bank.balance({
        address: client2.address,
        denom: "uscrt",
      })
    ).balance!.amount;

    console.log(
      `Iter ${i}: seat ${seat} wallet balance is: ${prevWalletBalanceP1} seat ${seat2} wallet balance is: ${prevWalletBalanceP2} bank balance is ${prevBankBalance}`
    );

    await test_game_flow(
      client,
      seat,
      bankCodeHash,
      bankAddress,
      gameCodeHash,
      gameAddress
    );

    const state = (await getTable(client, gameCodeHash, gameAddress)).state;
    assert(
      isPlayerTurn(state),
      `State expected to be player ${seat2} turn instead of ${JSON.stringify(
        state
      )}`
    );
    assert(
      (state as PT).PlayerTurn.player_seat === seat2,
      `State expected to be player ${seat2} turn instead of ${JSON.stringify(
        state
      )}`
    );

    await test_game_flow(
      client2,
      seat2,
      bankCodeHash,
      bankAddress,
      gameCodeHash,
      gameAddress
    );

    await roundup(
      client,
      prevBankBalance,
      [
        { seat: seat, prevBalance: prevWalletBalanceP1 },
        { seat: seat2, prevBalance: prevWalletBalanceP2 },
      ],
      bankCodeHash,
      bankAddress,
      gameCodeHash,
      gameAddress
    );
  }

  await stand(client, gameCodeHash, gameAddress, seat);
  await stand(client2, gameCodeHash, gameAddress, seat2);
}

async function test_game_flow_two_players_stand(
  client: SecretNetworkClient,
  client2: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string,
  gameCodeHash: string,
  gameAddress: string
) {
  const seat: number = 3;
  const seat2: number = 5;

  await sit(client, gameCodeHash, gameAddress, seat);

  for (let i: number = 0; i < 3; i++) {
    await sit(client2, gameCodeHash, gameAddress, seat2);

    const prevBankBalance: string = await queryBankBalance(
      client,
      bankCodeHash,
      bankAddress
    );

    const prevWalletBalanceP1: string = (
      await client.query.bank.balance({
        address: client.address,
        denom: "uscrt",
      })
    ).balance!.amount;

    await test_game_flow(
      client,
      seat,
      bankCodeHash,
      bankAddress,
      gameCodeHash,
      gameAddress
    );

    const state = (await getTable(client, gameCodeHash, gameAddress)).state;
    assert(
      isPlayerTurn(state),
      `State expected to be player ${seat2} turn instead of ${JSON.stringify(
        state
      )}`
    );
    assert(
      (state as PT).PlayerTurn.player_seat === seat2,
      `State expected to be player ${seat2} turn instead of ${JSON.stringify(
        state
      )}`
    );

    await stand(client2, gameCodeHash, gameAddress, seat2);

    await roundup(
      client,
      prevBankBalance,
      [{ seat: seat, prevBalance: prevWalletBalanceP1 }],
      bankCodeHash,
      bankAddress,
      gameCodeHash,
      gameAddress
    );
  }

  await stand(client, gameCodeHash, gameAddress, seat);
}

const delay = (ms: number) => new Promise((res) => setTimeout(res, ms));

async function test_game_flow_two_players_kick(
  client: SecretNetworkClient,
  client2: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string,
  gameCodeHash: string,
  gameAddress: string
) {
  const seat: number = 3;
  const seat2: number = 5;

  await sit(client, gameCodeHash, gameAddress, seat);
  await sit(client2, gameCodeHash, gameAddress, seat2);

  const prevBankBalance: string = await queryBankBalance(
    client,
    bankCodeHash,
    bankAddress
  );

  const prevWalletBalanceP1: string = (
    await client.query.bank.balance({
      address: client.address,
      denom: "uscrt",
    })
  ).balance!.amount;

  await test_game_flow(
    client,
    seat,
    bankCodeHash,
    bankAddress,
    gameCodeHash,
    gameAddress
  );

  const state = (await getTable(client, gameCodeHash, gameAddress)).state;
  assert(
    isPlayerTurn(state),
    `State expected to be player ${seat2} turn instead of ${JSON.stringify(
      state
    )}`
  );
  assert(
    (state as PT).PlayerTurn.player_seat === seat2,
    `State expected to be player ${seat2} turn instead of ${JSON.stringify(
      state
    )}`
  );

  await delay(6 * 60 * 1000);

  await kick(client2, gameCodeHash, gameAddress, seat2, client2.address);

  await roundup(
    client,
    prevBankBalance,
    [{ seat: seat, prevBalance: prevWalletBalanceP1 }],
    bankCodeHash,
    bankAddress,
    gameCodeHash,
    gameAddress
  );

  await stand(client, gameCodeHash, gameAddress, seat);
}

// async function test_gas_limits() {
//   // There is no accurate way to measure gas limits but it is actually very recommended to make sure that the gas that is used by a specific tx makes sense
// }

async function runTestFunction(
  tester: (
    client: SecretNetworkClient,
    bankCodeHash: string,
    bankAddress: string,
    gameCodeHash: string,
    gameAddress: string
  ) => void,
  client: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string,
  gameCodeHash: string,
  gameAddress: string
) {
  console.log(`Testing ${tester.name}`);
  await tester(client, bankCodeHash, bankAddress, gameCodeHash, gameAddress);
  console.log(`[SUCCESS] ${tester.name}`);
}

async function runTwoPlayersFunction(
  tester: (
    client: SecretNetworkClient,
    client2: SecretNetworkClient,
    bankCodeHash: string,
    bankAddress: string,
    gameCodeHash: string,
    gameAddress: string
  ) => void,
  client: SecretNetworkClient,
  client2: SecretNetworkClient,
  bankCodeHash: string,
  bankAddress: string,
  gameCodeHash: string,
  gameAddress: string
) {
  console.log(`Testing ${tester.name}`);
  await tester(
    client,
    client2,
    bankCodeHash,
    bankAddress,
    gameCodeHash,
    gameAddress
  );
  console.log(`[SUCCESS] ${tester.name}`);
}

(async () => {
  const [client, [gameCodeHash, gameAddress], [bankCodeHash, bankAddress]] =
    await initializeAndUploadContracts();

  let endpoint = "http://localhost:9091";
  let chainId = "secretdev-1";

  const client2 = await initializeClient(endpoint, chainId);

  await fillUpFromFaucet(client2, 1_000_000_000);

  await runTestFunction(
    test_bank_balance_on_initialization,
    client,
    bankCodeHash,
    bankAddress,
    gameCodeHash,
    gameAddress
  );
  await runTestFunction(
    test_bad_bid,
    client,
    bankCodeHash,
    bankAddress,
    gameCodeHash,
    gameAddress
  );
  await runTestFunction(
    test_normal_game_flow,
    client,
    bankCodeHash,
    bankAddress,
    gameCodeHash,
    gameAddress
  );

  await runTwoPlayersFunction(
    test_normal_game_flow_two_players,
    client,
    client2,
    bankCodeHash,
    bankAddress,
    gameCodeHash,
    gameAddress
  );

  await runTwoPlayersFunction(
    test_game_flow_two_players_stand,
    client,
    client2,
    bankCodeHash,
    bankAddress,
    gameCodeHash,
    gameAddress
  );

  // await runTwoPlayersFunction(
  //   test_game_flow_two_players_kick,
  //   client,
  //   client2,
  //   bankCodeHash,
  //   bankAddress,
  //   gameCodeHash,
  //   gameAddress
  // );
})();
