import sys
import matplotlib.pyplot as plt
import json

def convert_time(time: float) -> str:
    # Seconds to a nice [X]h[Y]' display.
    hours = int(time // 3600)
    minutes = int((time - hours * 3600) // 60)
    return f'{hours}h{minutes:0>2}\''

if __name__ == '__main__':
    try:
        target_file = sys.argv[1]
    except:
        print('Provide a target file')
        exit()

    fname = '.'.join(target_file.split('/')[-1].split('\\')[-1].split('.')[:-1])
    dest_fname = './results/parsed/' + fname

    # DATA READ

    file_content = json.loads('\n'.join(open(target_file)))

    names = file_content['names'][1:]

    data = list(map(lambda x: (int(x[0]), x[1][0], x[1][1]), file_content['assignments'].items()))

    optionals = [i[0] for i in data]
    times = [max(i[1]) / 3600 for i in data]
    optional_counts = [len(i[2]) for i in data]

    # PLOTTING
    fig, ax = plt.subplots()

    ax.scatter(times, optionals)

    for pos_x, pos_y in zip(times, optionals):
        ax.annotate(f'Optional: {pos_y}\nMax time: {round(pos_x, 1)}h', (pos_x, pos_y))

    ax.set_xlabel('Time (hours)')
    ax.set_ylabel('Number of optional destinations')
    plt.savefig(dest_fname + ".png", dpi=300, bbox_inches="tight")
    plt.show()

    # TEXT 
    res = ''
    for (optionals, times, assignments) in sorted(data, key=lambda x: x[0]):
        local_res = f'# Variant with {optionals} optional destinations\n\n'
        for day_idx, time in enumerate(times):
            local_res += f'## Day {day_idx + 1} ({convert_time(time)})\n'
            day_assignment = [names[place_idx] for place_idx, i in enumerate(assignments) if i == day_idx]
            for name in day_assignment:
                local_res += f'- {name}\n'
            if len(day_assignment) != 0:
                local_res += '\n'

        res += local_res + '\n'
    with open(dest_fname + '.md', 'w+') as file:
        file.write(res);
    # print(res)


