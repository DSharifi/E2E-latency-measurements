import numpy as np
import matplotlib.pyplot as plt


# Step 1: Read the files and convert lines to integers
def read_durations(file_path):
    try:
        with open(f"{file_path}.txt") as f:
            lines = f.readlines()
            return [int(x) for x in lines]
    except FileNotFoundError:
        with open(f"{file_path}_backup.txt") as f:
            lines = f.readlines()
            return [int(x) for x in lines]


durations_before = read_durations("before")
durations_after = read_durations("after")

# Step 2: Calculate percentiles
latencies = [0, 10, 20, 30, 40, 50, 60, 70, 75, 80, 85, 90, 95, 97, 99]
percentiles_before = np.percentile(durations_before, latencies)
percentiles_after = np.percentile(durations_after, latencies)

avg_before = np.mean(durations_before)
avg_after = np.mean(durations_after)

# Calculate the decrease in percentiles and average latency
decrease_percentiles = percentiles_before - percentiles_after
decrease_avg = avg_before - avg_after

# Calculate the percentage decrease in percentiles and average latency
percentage_decrease_percentiles = (decrease_percentiles / percentiles_before) * 100
percentage_decrease_avg = (decrease_avg / avg_before) * 100

# Step 3: Print the average latencies
print(f"Average Latency Before: {avg_before:.2f} ms")
print(f"Average Latency After: {avg_after:.2f} ms")
print(
    f"Decrease in Average Latency: {decrease_avg:.2f} ms ({percentage_decrease_avg:.2f}%)"
)

# Step 4: Print the percentiles in a table
print("E2E latencies:")
print(
    f"{'Percentile':<12}{'Before':<12}{'After':<12}{'Decrease':<12}{'Decrease (%)':<15}"
)
for p, before, after, dec, perc_dec in zip(
    latencies,
    percentiles_before,
    percentiles_after,
    decrease_percentiles,
    percentage_decrease_percentiles,
):
    print(f"{p:<12}{int(before):<12}{int(after):<12}{int(dec):<12}{perc_dec:.2f}%")

# Step 5: Plot the percentiles
plt.plot(latencies, percentiles_before, marker="o", label="before")
plt.plot(latencies, percentiles_after, marker="o", label="after")
plt.title("E2E latency for ingress messages to counter canister on `snjp` subnet.")
plt.xlabel("Percentile")
plt.ylabel("Duration (ms)")
plt.legend()
plt.grid(True)

# Add more lines on the y-axis
y_ticks = range(0, int(max(max(percentiles_before), max(percentiles_after))) + 100, 100)
plt.xticks(range(0, 101, 10))
plt.yticks(y_ticks)

plt.show()
