import React from 'react';
import styles from './styles/Hand.module.css';
import Card from './Card';
import { getConfigFileParsingDiagnostics } from 'typescript';

type HandProps = {
  title: string,
  cards: any[],
  isDealer: boolean,
  dealerScore: number,
};

const Hand: React.FC<HandProps> = ({ title, cards, isDealer, dealerScore }) => {
  const getCards = () => {
    if (cards.length === 0) {
      return (
        <div className={styles.cardContainer}>
          <Card key={0} value={"K"} suit={'♠'} nocard={true} hidden={false}/>
          <Card key={1} value={"K"} suit={'♠'} nocard={true} hidden={false}/>
        </div>
      );
    }

    if (cards.length === 1) {
      return (
        <div className={styles.cardContainer}>
          <Card key={0} value={"K"} suit={'♠'} nocard={!isDealer} hidden={isDealer}/>
          <Card key={1} value={cards[0].value} suit={cards[0].suit} nocard={cards[0].hidden} hidden={false}/>
        </div>
      );
    }

    return (
      <div className={styles.cardContainer}>
        {cards.map((card: any, index: number) => {
          return (
            <Card key={index} value={card.value} suit={card.suit} nocard={false} hidden={false}/>
          );
        })}
      </div>
    );
  }

  const getTitle = () => {
    if(isDealer) {
      if(dealerScore !== 0) {
        return (
          <h1 className={styles.title}>{title} (Score: {dealerScore})</h1>
        );
      } else {
        return (
          <h1 className={styles.title}>{title}</h1>
        );
      }
    } 

    return (
      <h1 className={styles.playerTitle}>{title}</h1>
    );
  }

  return (
    <div className={styles.handContainer}>
      {getTitle()}
      {getCards()}
    </div>
  );
}

export default Hand;