import React from 'react';
import styles from './styles/Status.module.css';

type StatusProps = {
  message: string,
  bet: number,
  balance: number
};

const Status: React.FC<StatusProps> = ({ message, bet, balance }) => {
  return (
    <div className={styles.statusContainer}>
      <div className={styles.status}>
        <h4 className={styles.value}>Balance is: {balance} u𝕊</h4>
      </div>
      <div className={styles.message}>
        <h2 className={styles.messageValue}>{message}</h2>
      </div>
      <div className={styles.bet}>
        <h1 className={styles.betValue}>Betting on: {bet} u𝕊</h1>
      </div>

    </div>
  );
}

export default Status;