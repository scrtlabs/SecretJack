import React, { useState, useEffect } from 'react';
import Status from './Status';
import Controls from './Controls';
import Hand from './Hand';
import Hands from './Hands';
import jsonData from '../deck.json';

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

const App: React.FC = () => {
  enum GameState {
    bet,
    init,
    userTurn,
    dealerTurn
  }

  enum Deal {
    p0,
    p1,
    p2,
    p3,
    p4,
    p5,
    dealer,
    hidden
  }

  enum Message {
    bet = 'Place a Bet!',
    hitStand = 'Hit or Stand?',
    bust = 'Bust!',
    userWin = 'You Win!',
    dealerWin = 'Dealer Wins!',
    tie = 'Tie!'
  }

  const data = JSON.parse(JSON.stringify(jsonData.cards));
  const [deck, setDeck]: any[] = useState(data);

  const [address, setAddress] = useState("");

  const m : Table = {
    players_count: 0,
    players: [],
    dealer_hand: null,
    state: ''
  };
  const [table, setTable]: Table = useState({

  });
  const [scores, setScores]: any[] = useState([0,0,0,0,0,0]);
  const [counts, setCounts]: any[] = useState([0,0,0,0,0,0]);

  const [dealerCards, setDealerCards]: any[] = useState([]);
  const [dealerScore, setDealerScore] = useState(0);
  const [dealerCount, setDealerCount] = useState(0);

  const [balance, setBalance] = useState(100);
  const [bet, setBet] = useState(0);

  const [gameState, setGameState] = useState(GameState.init);
  const [message, setMessage] = useState("");
  const [buttonState, setButtonState] = useState({
    hitDisabled: false,
    standDisabled: false,
    resetDisabled: true
  });

  useEffect(() => {
    if (gameState === GameState.init) {
      setGameState(GameState.userTurn);
      setMessage(Message.hitStand);
    }
  }, [gameState]);

  useEffect(() => {
    let temp_counts = counts;
    let temp_scores = scores;

    for (var i:number = 0; i < 6; i++) {
      temp_scores[i] = calculate(cards[i]);
      temp_counts[i] = cards[i].length;
    }

    setScores(temp_scores);
    setCounts(temp_counts);

    if (gameState === GameState.userTurn) {
      if (scores[0] === 21) {
        buttonState.hitDisabled = true;
        setButtonState({ ...buttonState });
      }
      else if (scores[0] > 21) {
        bust();
      }
    }
  });

  useEffect(() => {
    setDealerScore(calculate(dealerCards));
    setDealerCount(dealerCount + 1);
  }, [dealerCards]);

  useEffect(() => {
    if (gameState === GameState.dealerTurn) {
      if (dealerScore >= 17) {
        checkWin();
      }
      else {
        drawCard(Deal.dealer);
      }
    }
  }, [dealerCount]);

  const resetGame = () => {
    console.clear();
    setDeck(data);

    setCards([[], [], [], [], [], []]);
    setScores([0,0,0,0,0,0]);
    setCounts([0,0,0,0,0,0]);

    setDealerCards([]);
    setDealerScore(0);
    setDealerCount(0);

    setBet(0);

    setGameState(GameState.bet);
    setMessage(Message.bet);
    setButtonState({
      hitDisabled: false,
      standDisabled: false,
      resetDisabled: true
    });
  }

  const placeBet = (amount: number) => {
    setBet(amount);
    setBalance(Math.round((balance - amount) * 100) / 100);
    setGameState(GameState.init);
  }

  const drawCard = (dealType: Deal) => {
    if (deck.length > 0) {
      const randomIndex = Math.floor(Math.random() * deck.length);
      const card = deck[randomIndex];
      deck.splice(randomIndex, 1);
      setDeck([...deck]);
      console.log('Remaining Cards:', deck.length);
      switch (card.suit) {
        case 'spades':
          dealCard(dealType, card.value, '♠');
          break;
        case 'diamonds':
          dealCard(dealType, card.value, '♦');
          break;
        case 'clubs':
          dealCard(dealType, card.value, '♣');
          break;
        case 'hearts':
          dealCard(dealType, card.value, '♥');
          break;
        default:
          break;
      }
    }
    else {
      alert('All cards have been drawn');
    }
  }

  const dealCard = (dealType: Deal, value: string, suit: string) => {
    switch (dealType) {
      case Deal.p0:
        cards[0].push({ 'value': value, 'suit': suit, 'hidden': false });
        setCards(cards);
        break;
      case Deal.p1:
        cards[1].push({ 'value': value, 'suit': suit, 'hidden': false });
        setCards(cards);
        break;
      case Deal.p2:
        cards[2].push({ 'value': value, 'suit': suit, 'hidden': false });
        setCards(cards);
        break;
      case Deal.p3:
        cards[3].push({ 'value': value, 'suit': suit, 'hidden': false });
        setCards(cards);
        break;
      case Deal.p4:
        cards[4].push({ 'value': value, 'suit': suit, 'hidden': false });
        setCards(cards);
        break;
      case Deal.p5:
        cards[5].push({ 'value': value, 'suit': suit, 'hidden': false });
        setCards(cards);
        break;
      case Deal.dealer:
        dealerCards.push({ 'value': value, 'suit': suit, 'hidden': false });
        setDealerCards([...dealerCards]);
        break;
      case Deal.hidden:
        dealerCards.push({ 'value': value, 'suit': suit, 'hidden': true });
        setDealerCards([...dealerCards]);
        break;
      default:
        break;
    }
  }

  const revealCard = () => {
    dealerCards.filter((card: any) => {
      if (card.hidden === true) {
        card.hidden = false;
      }
      return card;
    });
    setDealerCards([...dealerCards])
  }

  const calculate = (cards: any[]) => {
    let total = 0;
    cards.forEach((card: any) => {
      if (card.hidden === false && card.value !== 'A') {
        switch (card.value) {
          case 'K':
            total += 10;
            break;
          case 'Q':
            total += 10;
            break;
          case 'J':
            total += 10;
            break;
          default:
            total += Number(card.value);
            break;
        }
      }
    });
    const aces = cards.filter((card: any) => {
      return card.value === 'A';
    });
    aces.forEach((card: any) => {
      if (card.hidden === false) {
        if ((total + 11) > 21) {
          total += 1;
        }
        else if ((total + 11) === 21) {
          if (aces.length > 1) {
            total += 1;
          }
          else {
            total += 11;
          }
        }
        else {
          total += 11;
        }
      }
    });

    return total;
  }

  const hit = () => {
    drawCard(Deal.p0);
    drawCard(Deal.p1);
    drawCard(Deal.p2);
    drawCard(Deal.p3);
    drawCard(Deal.p4);
    drawCard(Deal.p5);
  }

  const stand = () => {
    buttonState.hitDisabled = true;
    buttonState.standDisabled = true;
    buttonState.resetDisabled = false;
    setButtonState({ ...buttonState });
    setGameState(GameState.dealerTurn);
    revealCard();
  }

  const bust = () => {
    buttonState.hitDisabled = true;
    buttonState.standDisabled = true;
    buttonState.resetDisabled = false;
    setButtonState({ ...buttonState });
    setMessage(Message.bust);
  }

  const checkWin = () => {
    if (scores[0] > dealerScore || dealerScore > 21) {
      setBalance(Math.round((balance + (bet * 2)) * 100) / 100);
      setMessage(Message.userWin);
    }
    else if (dealerScore > scores[0]) {
      setMessage(Message.dealerWin);
    }
    else {
      setBalance(Math.round((balance + (bet * 1)) * 100) / 100);
      setMessage(Message.tie);
    }
  }

  return (
    <>
      <Status message={message} balance={balance} />
      <Controls
        balance={balance}
        gameState={gameState}
        buttonState={buttonState}
        betEvent={placeBet}
        hitEvent={hit}
        standEvent={stand}
        resetEvent={resetGame}
      />
      <Hand title={`Dealer's Hand (${dealerScore})`} cards={dealerCards} />
      <Hands cardsArr= {cards} scores= {scores}/>
    </>
  );
}

export default App;
