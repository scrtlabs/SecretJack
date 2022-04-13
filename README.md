# BlackJack over Secret Network

An implementation of a single-session BlackJack over Secret network.
In this game you will be able to double your money in less than a minute!

## Rules

1. The goal of blackjack is to beat the dealer's hand without going over 21.
2. Face cards are worth 10. Aces are worth 1 or 11, whichever makes a better hand.
3. Each player starts with two cards, one of the dealer's cards is hidden until the end.
4. To 'Hit' is to ask for another card. To 'Stand' is to hold your total and end your turn.
5. If you go over 21 you bust, and the dealer wins regardless of the dealer's hand.
6. If you are dealt 21 from the start (Ace & 10), you got a blackjack.
7. Blackjack means you win 1.25 the amount of your bet (WOW!!!!!)
8. Dealer will hit until his/her cards total 17 or higher.
9. Split can be done when you have two of the same card - the pair is split into two hands.
10. Splitting also doubles the bet, because each new hand is worth the original bet.
11. You can only split on the first move.
12. Aces can't be split.

# Design

## High Level Strategy

**_Note_**: In each and every time only one game session will be handled and only 6 seats will be available.
The players that are playing and the players that are waiting for a free seat can kick a player who won't be responsive in his turn for more than 1 minute.

The initial balance of the Bank will be 10 sSCRT.

**_Note_**: A bid will be approved only by checking the Bank's balance, if the bank can't afford to pay for a bet the player who tries to bet will receive a message and will get the opportunity to make another bet.

**_Example:_** When the balance of the available funds in the bank is 10 sSCRT the every player can bet 10 / 1.25 (Max win ratio) / 6 (Max amount of players) / 2 (For split) = 0.666 sSCRT.

## High Level Design

The game is built with 2 contracts in total.
The first contract (aka Bank), which is an administrative contract, will manage the funds of the game.
The money to the winner will be transferred by the bank and the money that was lost during the game will be transferred to the bank.
The second contract (aka Game) is the actual game manager it will interact both with the users (To manage the game session) and with the Bank to transfer money to and from the users.

## Flows

### User Entry

1. When a user is entered there are 2 options:
   - The user is a new user for which the balance is 0.
   - The user is an already known user for which the balance will be stored in the game contract's state.
2. After connecting to keplr wallet a Tx will be sent to get user viewing key with which user may seat in a free seat or manage his money (Deposit / Withdraw)
3. When a player took a seat a transaction will be sent to the game contract

### Player Money Management

1. Player need to deposit money to the contract with which he can play - only supported denom - usscrt (Tx to Game Contract)
2. At any time while not on a seat a player can withdraw his money (Tx to Game Contract)

### Round

#### While in round

1.  On each player's turn the following options will be available:
    For a seated player whose turn is now:

    - Bet (send Tx to Game contract which will expand the tx with a msg that will be send to the Bank contract the response will include the two cards which are randomly chosen also as the current state of the table, the dealer and all other players) - Before cards are dealt
    - Stand up (send Tx to game contract)

    The flow below if when the player decided to put a bid

    - If both of the cards are the same - Split (send Tx to Game contract which will expand the tx with a msg that will be send to the Bank contract) - The amount of your bet will be doubled (One for each deck)
    - Hit (send Tx to the Game contract) - Response will contain the new card dealt
    - Stand (send Tx to the Game contract)

    For a player or a player who is standing up:

    - When a player is not responsive in his turn for more than 1 minute - Kick - (send Tx to Game contract) - Player who was kicked will free his sit and his bet won't be returned!

2.  The round will continue until all players are on hold.
3.  The Game contract will expose the following queries:
    - GetUserBalance - get user balance - viewing key is required
    - GetTableStatus - get public table information
4.  The Bank contract will expose the following queries:
    - GetBankBalance - get the available funds in the bank - viewing key is required (????)

#### Round roundup

1.  Game roundup routine will be done by the Game contract when the contract will receive the last hold
2.  On roundup the Game contract will msg the Bank contract to give the money to the winners (If there are any)

### Dooms Day or Upgrade

Both of the contracts will have an administrative aspect for dooms-day and for upgrades:

1. Bank contract will have an administrative command to withdraw all of the money to an hard-coded wallet.
2. Game contract will have an administrative command to withdraw all the money stored in user balances back to the user's wallet.
